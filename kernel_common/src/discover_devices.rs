use core::{ptr::NonNull, str::FromStr};

use crate::{
    acpi_events::ACPI_GLOBALS,
    acpi_handler::{PCIE_MAPPINGS, SEGMENT_MAPPED_LEN},
    apic::{self, end_of_interrupt},
    async_executor::execute_future,
    hpet::HpetDelay,
    interrupts::assign_irq,
    memory::{alloc_phys, map_phys},
    paging::LeafMappingFlags,
    pat::STRONG_UNCACHEABLE_INDEX,
};
use acpi::aml::{
    InterruptModelUsed,
    namespace::AmlName,
    pci_routing::{PciRoutingTable, Pin},
};
use arbitrary_int::{traits::Integer, u3, u5};
use ez_ehci::{
    AnyEhci, EhciParts, InitDeviceBuffer, MappedMem, PCI_CLASS, PCI_PROG_IF, PCI_SUBCLASS,
    PeriodicFrameList, QueueHead, TryTakeOutput, new_ehci,
};
use ez_pci::{BarWithSize, MemoryBarAddrAndSizeU64, PciAccess, PciFunction};
use log::logger;
use spin::{Mutex, Once};
use x86_64::structures::idt::InterruptStackFrame;

struct EhciInfo {
    ehci: Mutex<ez_ehci::IrqHandler>,
    segment: u16,
    bus: u8,
    device: u5,
    function: u3,
}

static EHCI: Once<EhciInfo> = Once::new();

pub fn run() {
    let aml = &ACPI_GLOBALS.get().unwrap().aml_interpreter;
    let pci_routing_table =
        PciRoutingTable::from_prt_path(AmlName::from_str(r#"\_SB.PCI0._PRT"#).unwrap(), &aml)
            .unwrap();

    log::debug!("Got PCI Routing Table");
    for (segment, data) in PCIE_MAPPINGS.get().unwrap() {
        let mapped_mem = NonNull::slice_from_raw_parts(
            NonNull::new(data.virt as *mut _).unwrap(),
            SEGMENT_MAPPED_LEN.try_into().unwrap(),
        );
        let mut pci = unsafe { PciAccess::new_pcie(data.info, mapped_mem) };
        for bus_number in pci.known_buses() {
            let mut bus = pci.bus(bus_number);
            for device_number in u5::ZERO.value()..=u5::MAX.value() {
                let device_number = u5::new(device_number);
                let Some(mut device) = bus.device(device_number) else {
                    continue;
                };
                let possible_functions = device.possible_functions();
                for function_number in
                    possible_functions.start().value()..=possible_functions.end().value()
                {
                    let function_number = u3::new(function_number);
                    let Some(mut function) = device.function(function_number) else {
                        continue;
                    };
                }
            }
        }
    }
}
