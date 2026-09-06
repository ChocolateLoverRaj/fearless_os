extern crate alloc;
use core::time::Duration;

use uefi::{
    Identify,
    allocator::Allocator,
    boot::{
        OpenProtocolAttributes, OpenProtocolParams, SearchType, exit_boot_services,
        get_handle_for_protocol, image_handle, locate_handle_buffer, open_protocol,
        open_protocol_exclusive, stall,
    },
    mem::memory_map::MemoryMapOwned,
    prelude::*,
    proto::{
        acpi::AcpiTable,
        console::{gop::GraphicsOutput, serial::Serial},
        device_path::{
            acpi,
            text::{AllowShortcuts, DisplayOnly},
        },
        pci::root_bridge::PciRootBridgeIo,
    },
};
use x86_64::instructions::hlt;

#[global_allocator]
static GLOBAL_ALLOCATOR: Allocator = Allocator;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    log::info!("Hello from UEFI OS");

    let handle = locate_handle_buffer(SearchType::ByProtocol(
        &uefi::proto::pci::root_bridge::PciRootBridgeIo::GUID,
    ))
    .unwrap()[0];
    log::info!("Found handle");
    let mut pci = unsafe {
        open_protocol::<PciRootBridgeIo>(
            OpenProtocolParams {
                agent: image_handle(),
                controller: None,
                handle,
            },
            OpenProtocolAttributes::GetProtocol,
        )
        .unwrap()
    };
    log::info!("PCI root bridge: {:?}", pci);
    let pci_tree = pci.enumerate().unwrap();
    for addr in pci_tree {
        log::info!("PCI device: {addr:X?}");
    }

    let handle = get_handle_for_protocol::<Serial>().unwrap();
    let device_path = handle.device_path().unwrap();
    let device_path_str = alloc::string::String::from_utf16(
        device_path
            .to_string16(DisplayOnly(false), AllowShortcuts(false))
            .unwrap()
            .to_u16_slice(),
    )
    .unwrap();
    log::info!("Serial handle: {device_path_str}");

    let handle = get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut gop = open_protocol_exclusive::<GraphicsOutput>(handle).unwrap();
    for mode in gop.modes() {
        log::info!("Mode: {mode:#?}");
    }
    after_exit_boot_services(unsafe { exit_boot_services(None) })
}

fn after_exit_boot_services(memory_map: MemoryMapOwned) -> ! {
    loop {
        hlt();
    }
}
