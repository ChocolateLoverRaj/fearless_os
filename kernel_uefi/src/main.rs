#![no_std]
#![no_main]
extern crate alloc;
use core::ptr::NonNull;

mod logger;

use kernel_common::frame_buffer_embedded_graphics::{BufferingMode, FrameBufferEmbeddedGraphics};
use log::logger;
use uefi::{
    allocator::Allocator,
    boot::{exit_boot_services, get_handle_for_protocol, open_protocol_exclusive},
    mem::memory_map::{MemoryMap, MemoryMapMut, MemoryMapOwned},
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
            BufferingMode::DoubleBuffer,
        )
    };
    logger::switch_to_gop(buffer);
    logger().flush();
    // loop {}
    log::info!("Switched logging from text output to GOP frame buffer.");
    log::info!("Exiting boot services");
    logger().flush();
    after_exit_boot_services(unsafe { exit_boot_services(None) })
}

fn after_exit_boot_services(mut memory_map: MemoryMapOwned) -> ! {
    log::info!("Exited UEFI boot services");
    memory_map.sort();
    logger().flush();
    for entry in memory_map.entries() {
        log::info!("{entry:?}");
    }
    logger().flush();
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
