use acpi::{
    aml::resource::{InterruptPolarity, InterruptTrigger, IrqDescriptor},
    platform::{AcpiPlatform, InterruptModel},
};
use alloc::boxed::Box;
use common::{paging::LeafMappingFlags, pat::STRONG_UNCACHEABLE_INDEX};
use force_send_sync::Send as ForceSend;
use spin::{Mutex, Once};
use x2apic::{
    ioapic::{IoApic, IrqFlags, IrqMode, RedirectionTableEntry},
    lapic::{LocalApic, LocalApicBuilder, cpu_has_x2apic},
};

use crate::memory::map_phys;

static LOCAL_APIC: Once<Mutex<ForceSend<LocalApic>>> = Once::new();
static IO_APICS: Once<Mutex<Box<[(u32, IoApic)]>>> = Once::new();

// Handles Local and I/O APICs and 8259 PIC.
pub unsafe fn init(platform: &AcpiPlatform<impl acpi::Handler>) {
    unsafe { pic8259::ChainedPics::new_contiguous(0x20).disable() };

    let InterruptModel::Apic(apic) = &platform.interrupt_model else {
        panic!("Unknown interrupt model");
    };
    log::info!("APIC: {apic:#X?}");
    let mut local_apic_builder = LocalApicBuilder::new();
    local_apic_builder.error_vector(32);
    local_apic_builder.spurious_vector(33);
    local_apic_builder.timer_vector(34);
    if !cpu_has_x2apic() {
        // Map Local APIC
        let local_apic_virt_addr = map_phys(
            apic.local_apic_address,
            0x1000,
            LeafMappingFlags {
                writable: true,
                executable: false,
                user_mode_accessible: false,
                pat_index: STRONG_UNCACHEABLE_INDEX,
            },
        )
        .unwrap();
        local_apic_builder.set_xapic_base(local_apic_virt_addr);
    }
    let mut local_apic = local_apic_builder.build().unwrap();
    unsafe { local_apic.enable() };
    unsafe {
        local_apic.disable_timer();
    }
    LOCAL_APIC.call_once(|| Mutex::new(unsafe { force_send_sync::Send::new(local_apic) }));

    IO_APICS.call_once(|| {
        Mutex::new(
            {
                apic.io_apics.iter().map(|io_apic_info| {
                    let io_apic_virt_addr = map_phys(
                        io_apic_info.address.into(),
                        0x1000,
                        LeafMappingFlags {
                            writable: true,
                            user_mode_accessible: false,
                            executable: false,
                            pat_index: STRONG_UNCACHEABLE_INDEX,
                        },
                    )
                    .unwrap();
                    let mut io_apic = unsafe { IoApic::new(io_apic_virt_addr) };
                    let mut entry = RedirectionTableEntry::default();
                    // entry.set_dest(0);
                    // entry.set_mode(IrqMode::Fixed);
                    // entry.set_flags(IrqFlags::);
                    entry.set_vector(35);
                    unsafe { io_apic.set_table_entry(0x9, entry) };
                    unsafe { io_apic.enable_irq(0x9) };
                    (io_apic_info.global_system_interrupt_base, io_apic)
                })
            }
            .collect(),
        )
    });
}

pub unsafe fn end_of_interrupt() {
    let mut local_apic = LOCAL_APIC.get().unwrap().lock();
    unsafe { local_apic.end_of_interrupt() };
}

pub fn configure_ehci_interrupt(irq_descriptor: IrqDescriptor) {
    // TODO: there might be multiple irqs
    let gsi = irq_descriptor.irqs[0];
    let mut io_apics = IO_APICS.get().unwrap().lock();
    let (io_apic, entry_within) = io_apics
        .iter_mut()
        .find_map(|(base_gsi, io_apic)| {
            let entry_within = u8::try_from(gsi.checked_sub(*base_gsi)?).ok()?;
            if entry_within <= unsafe { io_apic.max_table_entry() } {
                Some((io_apic, entry_within))
            } else {
                None
            }
        })
        .unwrap();
    let mut entry = RedirectionTableEntry::default();
    entry.set_vector(36);
    entry.set_flags({
        let mut flags = IrqFlags::empty();
        if irq_descriptor.trigger == InterruptTrigger::Level {
            flags.insert(IrqFlags::LEVEL_TRIGGERED);
        }
        if irq_descriptor.polarity == InterruptPolarity::ActiveLow {
            flags.insert(IrqFlags::LOW_ACTIVE);
        }
        flags
    });
    unsafe { io_apic.set_table_entry(entry_within, entry) };
    unsafe { io_apic.enable_irq(entry_within) };
}
