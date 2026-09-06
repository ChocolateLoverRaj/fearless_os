use core::{
    cmp::Reverse,
    pin::Pin,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
    time::Duration,
};

use acpi::{AcpiTables, HpetInfo};
use alloc::{collections::binary_heap::BinaryHeap, sync::Arc};
use arbitrary_int::u5;
use common::{paging::LeafMappingFlags, pat::STRONG_UNCACHEABLE_INDEX};
use ez_hpet::{
    ApicDestMode, DeliveryMode, Hpet, HpetMemory, HpetTimerRef, InterruptConfig, InterruptTrigger,
    LEGACY_REPLACEMENT_ROUTES, RedirectionHint, TimerMode,
};
use futures::task::AtomicWaker;
use spin::{Mutex, Once};
use x86_64::instructions::interrupts::without_interrupts;

use crate::{
    acpi_handler::AcpiHandler, apic, config::CONFIG, interrupts::IrqAssignments, memory::map_phys,
};

pub static HPET: Once<Hpet<'static>> = Once::new();

#[derive(Debug)]
struct Timer {
    target_counter_value: u64,
    waker: AtomicWaker,
    timer_completed: AtomicBool,
}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.target_counter_value == other.target_counter_value
    }
}

impl Eq for Timer {}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.target_counter_value
            .partial_cmp(&other.target_counter_value)
    }
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.target_counter_value.cmp(&other.target_counter_value)
    }
}

static TIMER_LIST: Mutex<BinaryHeap<Reverse<Arc<Timer>>>> = Mutex::new(BinaryHeap::new());

pub fn init(acpi_tables: &AcpiTables<AcpiHandler>) {
    let hpet_info = HpetInfo::new(acpi_tables).unwrap();
    log::info!("HPET Info: {hpet_info:#X?}");
    // Technically we could support 32 bit but to keep code simpler we don't
    assert!(hpet_info.main_counter_is_64bits);

    let hpet_ptr = NonNull::new(
        map_phys(
            hpet_info.base_address.try_into().unwrap(),
            size_of::<HpetMemory>().try_into().unwrap(),
            LeafMappingFlags {
                writable: true,
                user_mode_accessible: false,
                executable: false,
                pat_index: STRONG_UNCACHEABLE_INDEX,
            },
        )
        .unwrap() as *mut _,
    )
    .unwrap();
    let mut hpet = unsafe { Hpet::new(hpet_ptr) };
    hpet.set_enable(true);

    // We're going to be very simple and select the first
    let enable_legacy_replacement =
        hpet.legacy_replacement_capable() && CONFIG.hpet_prefer_legacy_replacement;
    hpet.set_legacy_replacement_enabled(enable_legacy_replacement);
    let ticks_per_us = 1_000_000_000 / hpet.main_counter_tick_period();
    let microseconds_to_sleep = 1_000_000;
    let ticks_to_sleep = microseconds_to_sleep * u64::from(ticks_per_us);
    let counter = hpet.main_counter_value();
    let compare_value = counter + ticks_to_sleep;
    log::info!("timer value: {counter:#X}. compare value: {compare_value:#X}");
    // Only support 1 timer for now
    let mut timer = hpet.timer_mut(0);
    let supported_io_apic_interrupts = timer.supported_io_apic_interrupts();
    let supports_fsb_interrupts = timer.supports_fsb_interrupts();
    log::info!("HPET timer 0 supports IO-APIC interrupts: {supported_io_apic_interrupts:#b}");
    log::info!("HPET timer 0 supports FSB interrupts: {supports_fsb_interrupts:?}");
    if supports_fsb_interrupts {
        timer.configure_interrupt(InterruptConfig::Fsb {
            destination_mode: ApicDestMode::Physical,
            redirection_hint: RedirectionHint::DestId,
            destination_id: 0,
            interrupt_vector: IrqAssignments::Hpet as u8,
            delivery_mode: DeliveryMode::Fixed,
        });
        log::info!("HPET timer 0 configured to use FSB interrupt")
    } else if enable_legacy_replacement {
        timer.configure_interrupt(InterruptConfig::LegacyReplacment {
            trigger: InterruptTrigger::Edge,
        });
        apic::configure_hpet_interrupt(LEGACY_REPLACEMENT_ROUTES[0].apic_mapping);
        log::info!("HPET timer 0 routed to legacy replacment");
    } else {
        // Avoid interrupts 0..=15 because they can have legacy sources
        let io_apic_interrupt_to_use =
            u5::new(u8::try_from((supported_io_apic_interrupts & !0xF).trailing_zeros()).unwrap());
        timer.configure_interrupt(InterruptConfig::IoApic {
            io_apic_irq: io_apic_interrupt_to_use,
            trigger: InterruptTrigger::Edge,
        });
        apic::configure_hpet_interrupt(io_apic_interrupt_to_use.into());
        log::info!("HPET timer 0 routed to I/O irq {io_apic_interrupt_to_use:#X}");
    }
    timer.set_mode(TimerMode::Oneshot);
    // timer.set_comparator_value(compare_value);
    // timer.set_interrupt_enable(true);

    HPET.call_once(|| hpet);
}

// #[derive(Debug)]
// struct TimerWaker {
//     parent_waker: AtomicWaker,
//     wake_called: AtomicBool,
// }

// impl Wake for TimerWaker {
//     fn wake(self: alloc::sync::Arc<Self>) {
//         self.wake_by_ref();
//     }

//     fn wake_by_ref(self: &alloc::sync::Arc<Self>) {
//         self.wake_called.store(true, Ordering::Relaxed);
//         self.parent_waker.wake();
//     }
// }

/// Must be called with interrupts disabled
fn process_timers(timers: &mut BinaryHeap<Reverse<Arc<Timer>>>) {
    let hpet = HPET.get().unwrap();
    let mut timer = hpet.timer(0);
    loop {
        if let Some(Reverse(virtual_timer)) = timers.peek() {
            if hpet.main_counter_value() >= virtual_timer.target_counter_value {
                log::trace!("timer completed, setting as completed and calling waker");
                virtual_timer.timer_completed.store(true, Ordering::Relaxed);
                virtual_timer.waker.wake();
                timers.pop();
            } else {
                // Update timer value
                timer.set_comparator_value(virtual_timer.target_counter_value);
                timer.set_interrupt_enable(true);
                // Make sure that we didn't miss the edge interrupt
                if hpet.main_counter_value() < virtual_timer.target_counter_value {
                    // We can exit because even though interrupts are disabled right now, the HPET timer interrupt will fire
                    break;
                }
            }
        } else {
            // No more timers, disable timer
            timer.set_interrupt_enable(false);
            break;
        }
    }
}

/// Interrupts must be disabled when calling this
pub fn handle_irq() {
    log::trace!("handling HPET irq");
    // Currently only single core
    process_timers(&mut TIMER_LIST.try_lock().unwrap());
}

pub fn sleep(duration: Duration) -> TimerFuture {
    let hpet = HPET.get().unwrap();
    let main_counter_value = hpet.main_counter_value();
    // TODO: Support wrapping
    let ticks_to_add = u64::try_from(
        duration.as_nanos() * 1_000_000 / u128::from(hpet.main_counter_tick_period()),
    )
    .unwrap();
    let compare_value = main_counter_value + ticks_to_add;
    let timer = Arc::new(Timer {
        target_counter_value: compare_value,
        waker: AtomicWaker::new(),
        timer_completed: AtomicBool::new(false),
    });
    without_interrupts(|| {
        let mut timer_list = TIMER_LIST.lock();
        timer_list.push(Reverse(timer.clone()));
        process_timers(&mut timer_list);
    });

    TimerFuture { timer }
}

pub struct TimerFuture {
    timer: Arc<Timer>,
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        log::trace!("poll called for HPET timer future");
        self.timer.waker.register(cx.waker());
        if self.timer.timer_completed.load(Ordering::Relaxed) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl Drop for TimerFuture {
    fn drop(&mut self) {
        without_interrupts(|| {
            let mut timer_list = TIMER_LIST.lock();
            timer_list.retain(|virtual_timer| !Arc::ptr_eq(&virtual_timer.0, &self.timer));
            process_timers(&mut timer_list);
        })
    }
}
