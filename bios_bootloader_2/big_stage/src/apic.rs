use acpi::platform::{AcpiPlatform, InterruptModel};
use common::{paging::LeafMappingFlags, pat::STRONG_UNCACHEABLE_INDEX};
use force_send_sync::Send as ForceSend;
use spin::{Mutex, Once};
use x2apic::{
    ioapic::{IoApic, RedirectionTableEntry},
    lapic::{LocalApic, LocalApicBuilder, cpu_has_x2apic},
};

use crate::memory::map_phys;

static LOCAL_APIC: Once<Mutex<ForceSend<LocalApic>>> = Once::new();

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

    for io_apic in &apic.io_apics {
        let io_apic_virt_addr = map_phys(
            io_apic.address.into(),
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
    }
}

pub unsafe fn end_of_interrupt() {
    let mut local_apic = LOCAL_APIC.get().unwrap().lock();
    unsafe { local_apic.end_of_interrupt() };
}
