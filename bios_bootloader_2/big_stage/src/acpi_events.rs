use acpi::{platform::AcpiPlatform, registers::Pm1EventFlags};
use spin::Once;

use crate::acpi_handler::AcpiHandler;

static ACPI_PLATFORM: Once<AcpiPlatform<AcpiHandler>> = Once::new();

pub unsafe fn init(platform: AcpiPlatform<AcpiHandler>) {
    platform
        .registers
        .pm1_event_registers
        .set_enable_flags(Pm1EventFlags::GLOBAL_ENABLE | Pm1EventFlags::POWER_BUTTON);
    ACPI_PLATFORM.call_once(|| platform);
}

pub fn pending_events() -> Pm1EventFlags {
    ACPI_PLATFORM
        .get()
        .unwrap()
        .registers
        .pm1_event_registers
        .pending_events()
}

pub fn clear_events(events: Pm1EventFlags) {
    ACPI_PLATFORM
        .get()
        .unwrap()
        .registers
        .pm1_event_registers
        .clear_events(events);
}
