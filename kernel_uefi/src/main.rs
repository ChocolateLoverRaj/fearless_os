#![no_std]
#![no_main]
extern crate alloc;
use core::time::Duration;

mod logger;

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
        console::{
            gop::{BltOp, BltPixel, GraphicsOutput},
            serial::Serial,
        },
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

    logger::init();

    log::info!("Hello from UEFI OS");

    let handle = get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let result = open_protocol_exclusive::<GraphicsOutput>(handle);
    let mut gop = result.unwrap();
    gop.blt(BltOp::VideoFill {
        color: BltPixel::new(100, 150, 150),
        dest: (0, 0),
        dims: (100, 100),
    });
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

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    loop {
        hlt();
    }
}
