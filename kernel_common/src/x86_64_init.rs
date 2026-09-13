use acpi::{AcpiTables, platform::AcpiPlatform, sdt::fadt::Fadt};

use crate::{
    acpi_events,
    acpi_handler::{self, AcpiHandler},
    apic,
    config::CONFIG,
    hpet, interrupts,
};

pub unsafe fn init(offset_map_virt_addr: u64, rsdp: usize) {
    // interrupts::init();

    let acpi_handler = AcpiHandler {
        offset_map_virt_addr,
    };

    let acpi_tables = unsafe { AcpiTables::from_rsdp(acpi_handler, rsdp) }.unwrap();
    for (_phys_addr, table) in acpi_tables.table_headers() {
        let signature = table.signature;
        log::info!("ACPI Table: {signature}.");
    }
    acpi_handler::init(&acpi_tables);

    let platform = AcpiPlatform::new(acpi_tables, acpi_handler).unwrap();
    log::info!("Got platform");

    unsafe { apic::init(&platform) };

    if CONFIG.enter_acpi_mode {
        platform.enter_acpi_mode().unwrap();
        log::info!("Entered ACPI mode");
    }

    let fadt = platform.tables.find_table::<Fadt>().unwrap();
    let sci_interrupt = fadt.sci_interrupt;
    log::info!("SCI Interrupt IRQ: {sci_interrupt:#X}");

    hpet::init(&platform.tables);

    // execute_future(async {
    //     loop {
    //         sleep(Duration::from_secs(1)).await;
    //         log::info!("after sleep");
    //     }
    // });

    unsafe { acpi_events::init(platform) };
}
