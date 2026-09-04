use core::{ptr::NonNull, str::FromStr};

use acpi::aml::{self, namespace::AmlName, pci_routing::PciRoutingTable};
use arbitrary_int::{traits::Integer, u3, u5};
use common::{paging::LeafMappingFlags, pat::STRONG_UNCACHEABLE_INDEX};
use ez_ehci::{
    AnyEhci, InitDeviceBuffer, InitDeviceError, MappedMem, PCI_CLASS, PCI_PROG_IF, PCI_SUBCLASS,
    PeriodicFrameList, RunOutput, TryTakeOutput, new_ehci,
};
use ez_pci::{BarWithSize, MemoryBarAddrAndSizeU64, PciAccess, PciFunction};
use log::logger;
use x86_64::instructions::hlt;

use crate::{
    acpi_events::ACPI_GLOBALS,
    acpi_handler::{PCIE_MAPPINGS, SEGMENT_MAPPED_LEN},
    memory::{alloc_phys, map_phys},
};

pub fn run() -> ! {
    let aml = aml::Interpreter::new_from_platform(&ACPI_GLOBALS.get().unwrap().platform).unwrap();
    aml.initialize_namespace();
    let pci_routing_table =
        PciRoutingTable::from_prt_path(AmlName::from_str(r#"\_SB.PCI0._PRT"#).unwrap(), &aml)
            .unwrap();

    log::debug!("PCI Routing Table: {pci_routing_table:#X?}");
    for (segment, data) in PCIE_MAPPINGS.lock().iter() {
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
                    let Some(mut function) = device.function(u3::new(function_number)) else {
                        continue;
                    };
                    if function.class_code() == PCI_CLASS
                        && function.sub_class() == PCI_SUBCLASS
                        && function.prog_if() == PCI_PROG_IF
                    {
                        log::info!("Found eHCI PCI device");
                        let interrupt_info = function.interrupt_info().unwrap();
                        log::info!("interrupt info: {interrupt_info:#X?}");
                        let bar = function.read_bar_with_size(0).unwrap().unwrap();
                        log::info!("bar: {bar:#X?}");

                        let command = function
                            .command()
                            .with_bus_master(true)
                            .with_memory_space(true)
                            .with_interrupt_disable(false);
                        function.set_command(command);

                        let BarWithSize::Memory(bar) = bar else {
                            panic!()
                        };
                        let MemoryBarAddrAndSizeU64 { addr, size } =
                            bar.addr_and_size.addr_and_size_u64();
                        let mapped_bar = NonNull::slice_from_raw_parts(
                            NonNull::new(
                                map_phys(
                                    addr,
                                    size,
                                    LeafMappingFlags {
                                        writable: true,
                                        executable: false,
                                        user_mode_accessible: false,
                                        pat_index: if bar.prefetchable {
                                            // WRITE_THROUGH_INDEX
                                            STRONG_UNCACHEABLE_INDEX
                                        } else {
                                            STRONG_UNCACHEABLE_INDEX
                                        },
                                    },
                                )
                                .unwrap() as *mut u8,
                            )
                            .unwrap(),
                            size.try_into().unwrap(),
                        );
                        struct MyPciAccess<'a> {
                            function: PciFunction<'a>,
                        }
                        impl<'a> ez_ehci::PciAccess for MyPciAccess<'a> {
                            fn read_u32(&mut self, offset: u8) -> u32 {
                                self.function.read(offset.into())
                            }
                            fn write_u8(&mut self, offset: u8, value: u8) {
                                self.function.write(offset.into(), value);
                            }
                            fn write_u16(&mut self, offset: u8, value: u16) {
                                self.function.write(offset.into(), value);
                            }
                            fn write_u32(&mut self, offset: u8, value: u32) {
                                self.function.write(offset.into(), value);
                            }
                        }
                        let mut pci_access = MyPciAccess { function };
                        let ehci = match unsafe { new_ehci(mapped_bar, &mut pci_access) } {
                            AnyEhci::OsOwned(ehci) => ehci,
                            AnyEhci::BiosOwned(ehci) => {
                                log::info!("eHCI owned by BIOS");
                                let mut ehci = ehci.take_ownership();
                                let ehci = loop {
                                    match ehci.try_take() {
                                        TryTakeOutput::Taken(ehci) => break ehci,
                                        TryTakeOutput::NotYet(new_ehci) => {
                                            ehci = new_ehci;
                                        }
                                    }
                                };
                                log::info!("took ownership of eHCI");
                                ehci
                            }
                        };
                        let mut function = pci_access.function;
                        // log::info!("Getting IRQ descriptor");
                        // let route = pci_routing_table.route(
                        //     device_number.into(),
                        //     function_number.value().into(),
                        //     match interrupt_info.interrupt_pin {
                        //         0x1 => Pin::IntA,
                        //         0x2 => Pin::IntB,
                        //         0x3 => Pin::IntC,
                        //         0x4 => Pin::IntD,
                        //         interrupt_pin => {
                        //             panic!("unknown interrupt pin: {interrupt_pin}")
                        //         }
                        //     },
                        //     &aml,
                        // );
                        // log::info!("got route: {route:?}");
                        // let irq_descriptor = route.unwrap();
                        // log::info!("eHCI irq descriptor: {irq_descriptor:#X?}");
                        // apic::configure_ehci_interrupt(irq_descriptor);

                        let ehci_flags = LeafMappingFlags {
                            writable: true,
                            executable: false,
                            user_mode_accessible: false,
                            pat_index: STRONG_UNCACHEABLE_INDEX,
                        };
                        let mem = alloc_phys(
                            size_of::<PeriodicFrameList>().try_into().unwrap(),
                            align_of::<PeriodicFrameList>().try_into().unwrap(),
                        )
                        .unwrap();
                        let ptr = NonNull::new(
                            map_phys(
                                mem,
                                size_of::<PeriodicFrameList>().try_into().unwrap(),
                                ehci_flags,
                            )
                            .unwrap() as *mut _,
                        )
                        .unwrap();
                        let mut ehci = ehci.init(MappedMem {
                            phys_addr: mem.try_into().unwrap(),
                            ptr: ptr,
                        });
                        log::info!("eHCI initialized");
                        logger().flush();
                        // x86_64::instructions::interrupts::enable();
                        loop {
                            log::info!("running eHCI");
                            let device = loop {
                                match ehci.run() {
                                    RunOutput::Idle => {
                                        log::info!("idling (halting with interrupt enabled).");
                                        loop {
                                            hlt();
                                        }
                                    }
                                    RunOutput::NewDevice(device) => break device,
                                };
                            };
                            log::info!("New device: {device:?}");
                            let mem = alloc_phys(
                                size_of::<InitDeviceBuffer>().try_into().unwrap(),
                                align_of::<InitDeviceBuffer>().try_into().unwrap(),
                            )
                            .unwrap();
                            let ptr = NonNull::new(
                                map_phys(
                                    mem,
                                    size_of::<InitDeviceBuffer>().try_into().unwrap(),
                                    ehci_flags,
                                )
                                .unwrap() as *mut _,
                            )
                            .unwrap();
                            let command = function.command();
                            let pci_status = function.status();
                            log::info!("{command:#X?} {pci_status:#X?}");
                            match ehci.init_device(
                                device.port,
                                MappedMem {
                                    phys_addr: mem.try_into().unwrap(),
                                    ptr,
                                },
                            ) {
                                Ok(_) => {}
                                Err(e) => {
                                    if let InitDeviceError::HostSystemError = e {
                                        let command = function.command();
                                        let pci_status = function.status();
                                        log::info!("{command:#X?} {pci_status:#X?}");
                                    }
                                    panic!("{e:?}")
                                }
                            };
                        }
                    }
                }
            }
        }
    }
    loop {
        hlt();
    }
}
