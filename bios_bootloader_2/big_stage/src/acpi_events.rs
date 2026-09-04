use acpi::{aml, platform::AcpiPlatform, registers::Pm1EventFlags, sdt::fadt::Fadt};
use spin::Once;

use crate::acpi_handler::AcpiHandler;

pub struct AcpiGlobals {
    pub platform: AcpiPlatform<AcpiHandler>,
    pub aml_interpreter: aml::Interpreter<AcpiHandler>,
}

pub static ACPI_GLOBALS: Once<AcpiGlobals> = Once::new();

pub unsafe fn init(platform: AcpiPlatform<AcpiHandler>) {
    let fadt = platform.tables.find_table::<Fadt>().unwrap();
    let flags = fadt.flags;
    let is_hw_reduced = flags.system_is_hw_reduced_acpi();
    log::info!("ACPI: is_hw_reduced = {is_hw_reduced}.");

    platform
        .registers
        .pm1_event_registers
        .set_enable_flags(Pm1EventFlags::GLOBAL_ENABLE | Pm1EventFlags::POWER_BUTTON);
    let aml_interpreter = aml::Interpreter::new_from_platform(&platform).unwrap();
    aml_interpreter.initialize_namespace();
    ACPI_GLOBALS.call_once(|| AcpiGlobals {
        platform,
        aml_interpreter,
    });
}

pub fn pending_events() -> Pm1EventFlags {
    ACPI_GLOBALS
        .get()
        .unwrap()
        .platform
        .registers
        .pm1_event_registers
        .pending_events()
}

pub fn clear_events(events: Pm1EventFlags) {
    ACPI_GLOBALS
        .get()
        .unwrap()
        .platform
        .registers
        .pm1_event_registers
        .clear_events(events);
}

pub fn platform() -> &'static AcpiPlatform<AcpiHandler> {
    &ACPI_GLOBALS.get().unwrap().platform
}
