use core::{fmt::Write, ptr::NonNull};

use acpi::sdt::spcr::Spcr;
use alloc::boxed::Box;
use arbitrary_int::{traits::Integer, u3, u5};
use ez_pci::PciAccess;

use crate::{
    acpi_events::ACPI_GLOBALS,
    acpi_handler::{PCIE_MAPPINGS, SEGMENT_MAPPED_LEN},
    pci_serial, spcr,
};

pub fn init() -> Option<Box<dyn Write + Send>> {
    // First check for SPCR
    if let Some(spcr) = ACPI_GLOBALS
        .get()
        .unwrap()
        .platform
        .tables
        .find_table::<Spcr>()
    {
        match spcr::init(&spcr) {
            Ok(uart) => return Some(Box::new(uart)),
            Err(e) => {
                log::warn!("Error using SPCR: {e:#X?}");
            }
        }
    }

    for data in PCIE_MAPPINGS.get().unwrap().values() {
        let mapped_mem = NonNull::slice_from_raw_parts(
            NonNull::new(data.virt as *mut _).unwrap(),
            SEGMENT_MAPPED_LEN.try_into().unwrap(),
        );
        let mut pci = unsafe { PciAccess::new_pcie(data.info, mapped_mem) };
        for bus in pci.known_buses() {
            let mut bus = pci.bus(bus);
            for device_number in u5::ZERO.value()..=u5::MAX.value() {
                let Some(mut device) = bus.device(u5::new(device_number)) else {
                    continue;
                };
                let possible_functions = device.possible_functions();
                for function_number in
                    possible_functions.start().value()..=possible_functions.end().value()
                {
                    let Some(function) = device.function(u3::new(function_number)) else {
                        continue;
                    };
                    match pci_serial::init(function) {
                        Ok(Some(uart)) => {
                            return Some(uart);
                        }
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!("Error using PCI serial: {e:#X?}");
                        }
                    }
                }
            }
        }
    }
    None
}
