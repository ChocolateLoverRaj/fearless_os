#![no_std]
#![no_main]
extern crate alloc;
use core::ptr::NonNull;

mod logger;

use kernel_common::frame_buffer_embedded_graphics::FrameBufferEmbeddedGraphics;
use uefi::{
    allocator::Allocator,
    boot::{exit_boot_services, get_handle_for_protocol, open_protocol_exclusive},
    mem::memory_map::{MemoryMap, MemoryMapOwned},
    prelude::*,
    proto::console::gop::GraphicsOutput,
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

    let mut gop = open_protocol_exclusive::<GraphicsOutput>(handle).unwrap();
    let buffer = unsafe {
        FrameBufferEmbeddedGraphics::new(
            NonNull::new(gop.frame_buffer().as_mut_ptr().cast()).unwrap(),
            gop.current_mode_info().try_into().unwrap(),
            true,
        )
    };
    logger::switch_to_gop(buffer);
    log::info!("Switched logging from text output to GOP frame buffer.");
    after_exit_boot_services(unsafe { exit_boot_services(None) })
}

fn after_exit_boot_services(memory_map: MemoryMapOwned) -> ! {
    log::info!("Exited UEFI boot services");
    for entry in memory_map.entries() {
        log::info!("{entry:?}");
    }
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
