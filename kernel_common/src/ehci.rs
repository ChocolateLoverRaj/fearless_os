use core::{ptr::NonNull, str::FromStr};

use crate::{
    acpi_events::ACPI_GLOBALS,
    acpi_handler::{PCIE_MAPPINGS, SEGMENT_MAPPED_LEN},
    apic::{self, end_of_interrupt},
    async_executor::execute_future,
    hpet::HpetDelay,
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
    AnyEhci, InitDeviceBuffer, InitializedEhci, MappedMem, PCI_CLASS, PCI_PROG_IF, PCI_SUBCLASS,
    PeriodicFrameList, TryTakeOutput, new_ehci,
};
use ez_pci::{BarWithSize, MemoryBarAddrAndSizeU64, PciAccess, PciFunction};
use log::logger;
use spin::Once;
use x86_64::structures::idt::InterruptStackFrame;

struct EhciInfo {
    ehci: InitializedEhci,
    segment: u16,
    bus: u8,
    device: u5,
    function: u3,
}

static EHCI: Once<EhciInfo> = Once::new();

pub fn run() {
    let aml = &ACPI_GLOBALS.get().unwrap().aml_interpreter;
    aml.set_interrupt_model_used(InterruptModelUsed::ApicMode)
        .unwrap();
    let pci_routing_table =
        PciRoutingTable::from_prt_path(AmlName::from_str(r#"\_SB.PCI0._PRT"#).unwrap(), &aml)
            .unwrap();

    log::debug!("Got PCI Routing Table");
    let ehci_flags = LeafMappingFlags {
        writable: true,
        executable: false,
        user_mode_accessible: false,
        pat_index: STRONG_UNCACHEABLE_INDEX,
    };
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
                    if function.class_code() == PCI_CLASS
                        && function.sub_class() == PCI_SUBCLASS
                        && function.prog_if() == PCI_PROG_IF
                    {
                        log::info!(
                            "Found eHCI PCI device at bus {bus_number:x} device {device_number:x} function {function_number:x}"
                        );
                        let interrupt_info = function.interrupt_info().unwrap();
                        log::info!("interrupt info: {interrupt_info:#X?}");
                        let bar = function.read_bar_with_size(0).unwrap().unwrap();
                        log::info!("bar: {bar:#X?}");
                        log::info!("Getting IRQ descriptor");
                        let route = pci_routing_table.route(
                            device_number.into(),
                            function_number.value().into(),
                            Pin::from_pci_interrupt_pin(interrupt_info.interrupt_pin)
                                .unwrap()
                                .unwrap(),
                            &aml,
                        );
                        log::info!("got route: {route:?}");
                        let irq_descriptor = route.unwrap();
                        log::info!("eHCI irq descriptor: {irq_descriptor:#X?}");

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

                        apic::configure_ehci_interrupt(irq_descriptor);

                        let mem_0 = alloc_phys(
                            size_of::<PeriodicFrameList>().try_into().unwrap(),
                            align_of::<PeriodicFrameList>().try_into().unwrap(),
                        )
                        .unwrap();
                        let ptr_0 = NonNull::new(
                            map_phys(
                                mem_0,
                                size_of::<PeriodicFrameList>().try_into().unwrap(),
                                ehci_flags,
                            )
                            .unwrap() as *mut _,
                        )
                        .unwrap();
                        let mem_1 = alloc_phys(
                            size_of::<QueueHead>().try_into().unwrap(),
                            align_of::<QueueHead>().try_into().unwrap(),
                        )
                        .unwrap();
                        let ptr_1 = NonNull::new(
                            map_phys(
                                mem_1,
                                size_of::<PeriodicFrameList>().try_into().unwrap(),
                                ehci_flags,
                            )
                            .unwrap() as *mut _,
                        )
                        .unwrap();
                        let ehci = ehci.init(MappedMem {
                            phys_addr: mem_0.try_into().unwrap(),
                            ptr: ptr_0,
                        });
                        log::info!("eHCI initialized");
                        EHCI.call_once(|| EhciInfo {
                            ehci,
                            segment: *segment,
                            bus: bus_number,
                            device: device_number,
                            function: function_number,
                        });
                    }
                }
            }
        }
    }

    if let Some(ehci) = EHCI.get() {
        let ehci = &ehci.ehci;
        execute_future(async {
            logger().flush();
            loop {
                log::info!("running eHCI");
                let device = ehci.run().await;
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
                match ehci
                    .init_device(
                        device.port,
                        MappedMem {
                            phys_addr: mem.try_into().unwrap(),
                            ptr,
                        },
                        &mut HpetDelay,
                    )
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        panic!("{e:?}")
                    }
                };
            }
        });
    }
}

pub extern "x86-interrupt" fn ehci_interrupt_handler(_stack_frame: InterruptStackFrame) {
    if let Some(ehci) = EHCI.get() {
        let data = PCIE_MAPPINGS.get().unwrap().get(&ehci.segment).unwrap();
        let mapped_mem = NonNull::slice_from_raw_parts(
            NonNull::new(data.virt as *mut _).unwrap(),
            SEGMENT_MAPPED_LEN.try_into().unwrap(),
        );
        let mut pci = unsafe { PciAccess::new_pcie(data.info, mapped_mem) };
        let interrupt_status = pci
            .bus(ehci.bus)
            .device(ehci.device)
            .unwrap()
            .function(ehci.function)
            .unwrap()
            .status()
            .interrupt_status();
        // On Lenovo Ideapad Z560, this IRQ is called even when the PCI status has no interrupt.
        // This seems to be a hardware bug and we ignore these extra IRQs.
        if interrupt_status {
            ehci.ehci.handle_interrupt();
        } else {
            // log::warn!("received eHCI interrupt when PCI status indicates no interrupt");
        }
    }
    unsafe { end_of_interrupt() };
}
