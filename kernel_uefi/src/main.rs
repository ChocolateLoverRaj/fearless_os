#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
extern crate alloc;
use core::ptr::NonNull;

mod logger;
mod memory;

use alloc::boxed::Box;
use kernel_common::{
    frame_buffer_embedded_graphics::{BufferingMode, FrameBufferEmbeddedGraphics},
    interrupts,
};
use log::logger;
use uefi::{
    boot::{exit_boot_services, get_handle_for_protocol, open_protocol_exclusive},
    mem::memory_map::{MemoryMap, MemoryMapMut, MemoryMapOwned},
    prelude::*,
    proto::console::gop::GraphicsOutput,
    table::{
        cfg::{self, ConfigTableEntry},
        system_table_raw,
    },
};
use x86_64::instructions::{hlt, interrupts::int3};

#[derive(Debug, Clone, Copy)]
enum RsdpType {
    Rsdp,
    Xsdt,
}

#[derive(Debug, Clone, Copy)]
struct RsdpInfo {
    _type: RsdpType,
    addr: usize,
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    logger::init();

    log::info!("Hello from UEFI OS");
    logger().flush();

    let handle = get_handle_for_protocol::<GraphicsOutput>().unwrap();

    // TODO: We could show a screen that let's the user set the resolution.
    // Otherwise we would need a GPU driver to change the resolution later.
    //
    // let gop = unsafe {
    //     open_protocol::<GraphicsOutput>(
    //         OpenProtocolParams {
    //             handle,
    //             agent: image_handle(),
    //             controller: None,
    //         },
    //         OpenProtocolAttributes::GetProtocol,
    //     )
    // }
    // .unwrap();
    // for mode in gop.modes() {
    //     log::info!("Mode: {mode:#?}");
    // }
    // drop(gop);

    log::info!("Switching to GOP frame buffer");
    let mut gop = open_protocol_exclusive::<GraphicsOutput>(handle).unwrap();
    // let mode_to_use_index = choose_resolution(|| gop.modes().map(|mode| mode.into()))
    //     .expect("must have at least 1 mode");
    // let mode = gop.modes().nth(mode_to_use_index).unwrap();
    // Changing mode is disabled because in qemu this results in diagnoally skewed output, even when calling blt
    // gop.set_mode(&mode).unwrap();
    // gop.blt(uefi::proto::console::gop::BltOp::VideoFill {
    //     color: uefi::proto::console::gop::BltPixel::new(50, 100, 250),
    //     dest: (0, 0),
    //     dims: (100, 100),
    // })
    // .unwrap();
    let buffer = unsafe {
        FrameBufferEmbeddedGraphics::new(
            NonNull::new(gop.frame_buffer().as_mut_ptr().cast()).unwrap(),
            (&gop.current_mode_info()).try_into().unwrap(),
            BufferingMode::Direct,
        )
    };
    logger::switch_to_gop(buffer);
    logger().flush();
    // loop {}
    log::info!("Switched logging from text output to GOP frame buffer.");

    let system_table = system_table_raw().unwrap();
    let system_table = unsafe { system_table.as_ref() };
    let config_table = NonNull::slice_from_raw_parts(
        NonNull::new(system_table.configuration_table).unwrap(),
        system_table.number_of_configuration_table_entries,
    );
    let config_table = unsafe { config_table.as_ref() };

    let rsdp_info = config_table
        .iter()
        .find_map(|entry| {
            if entry.vendor_guid == ConfigTableEntry::ACPI2_GUID {
                Some(RsdpInfo {
                    _type: RsdpType::Xsdt,
                    addr: entry.vendor_table.addr(),
                })
            } else if entry.vendor_guid == ConfigTableEntry::ACPI_GUID {
                Some(RsdpInfo {
                    _type: RsdpType::Rsdp,
                    addr: entry.vendor_table.addr(),
                })
            } else {
                None
            }
        })
        .unwrap();

    log::info!("Exiting boot services");
    logger().flush();
    after_exit_boot_services(unsafe { exit_boot_services(None) }, rsdp_info)
}

fn after_exit_boot_services(mut memory_map: MemoryMapOwned, rsdp_info: RsdpInfo) -> ! {
    log::info!("Exited UEFI boot services");
    log::info!("RSDP info: {rsdp_info:#X?}");
    logger().flush();
    memory_map.sort();
    for entry in memory_map.entries() {
        log::debug!("{entry:?}");
    }
    log::debug!("Entries count: {}", memory_map.len());
    logger().flush();
    unsafe { memory::init(memory_map) };
    log::info!("Initialized memory");
    logger().flush();
    interrupts::init();
    log::info!("Initialized interrupts");
    logger().flush();
    int3();

    loop {
        hlt();
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    logger().flush();
    loop {
        hlt();
    }
}
