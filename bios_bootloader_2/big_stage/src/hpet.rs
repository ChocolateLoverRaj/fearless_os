use core::ptr::NonNull;

use acpi::{AcpiTables, HpetInfo};
use arbitrary_int::u5;
use common::{paging::LeafMappingFlags, pat::STRONG_UNCACHEABLE_INDEX};
use ez_hpet::{Hpet, HpetMemory, HpetTimerRef, InterruptConfig, InterruptTrigger, TimerMode};
use spin::Once;
use x86_64::instructions::{hlt, interrupts};

use crate::{acpi_handler::AcpiHandler, apic, memory::map_phys};

pub static HPET: Once<Hpet<'static>> = Once::new();

pub fn init(acpi_tables: &AcpiTables<AcpiHandler>) {
    let hpet_info = HpetInfo::new(acpi_tables).unwrap();
    log::info!("HPET Info: {hpet_info:#X?}");
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
    hpet.set_legacy_replacement_enabled(false);
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
    // Avoid interrupts 0..=15 because they can have legacy sources
    let io_apic_interrupt_to_use =
        u5::new(u8::try_from((supported_io_apic_interrupts & !0xF).trailing_zeros()).unwrap());
    // let io_apic_interrupt_to_use = u5::new(2);
    timer.configure_interrupt(InterruptConfig::IoApic(io_apic_interrupt_to_use));
    let configuration = timer.interrupt_cfg();
    log::info!("interrupt configuration: {configuration:#?}");

    timer.set_comparator_value(compare_value);
    timer.set_interrupt_enable(true);
    timer.set_trigger(InterruptTrigger::Level);
    timer.set_mode(TimerMode::Oneshot);

    apic::configure_hpet_interrupt(io_apic_interrupt_to_use.into());

    log::info!("routed to interrupt {io_apic_interrupt_to_use}");
    // For testing
    interrupts::enable();
    loop {
        hlt();
    }

    HPET.call_once(|| hpet);
}
