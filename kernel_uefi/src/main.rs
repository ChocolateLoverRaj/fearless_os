#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
extern crate alloc;
use core::{ptr::NonNull, time::Duration};

mod global_allocator;
mod logger;
mod memory;

use kernel_common::{
    async_executor::execute_future,
    frame_buffer_embedded_graphics::{BufferingMode, FrameBufferEmbeddedGraphics},
    hpet::sleep,
    interrupts, serial, x86_64_init,
};
use log::logger;
use uefi::{
    boot::{exit_boot_services, get_handle_for_protocol, open_protocol_exclusive},
    mem::memory_map::{MemoryMap, MemoryMapMut, MemoryMapOwned},
    prelude::*,
    proto::console::gop::GraphicsOutput,
    table::{cfg::ConfigTableEntry, system_table_raw},
};
use x86_64::instructions::{hlt, interrupts::int3};

use crate::{global_allocator::GLOBAL_ALLOCATOR, memory::OFFSET_MAP_VIRT_ADDR};

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
            BufferingMode::DoubleBuffer,
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

    let rsdp = config_table
        .iter()
        .find_map(|entry| {
            if entry.vendor_guid == ConfigTableEntry::ACPI_GUID {
                Some(entry.vendor_table.addr())
            } else {
                None
            }
        })
        .unwrap();

    log::info!("Exiting boot services");
    logger().flush();
    GLOBAL_ALLOCATOR.disable_uefi();
    after_exit_boot_services(unsafe { exit_boot_services(None) }, rsdp)
}

fn after_exit_boot_services(mut memory_map: MemoryMapOwned, rsdp: usize) -> ! {
    log::info!("Exited UEFI boot services");
    log::debug!("RSDP: {rsdp:#X?}");
    logger().flush();
    memory_map.sort();
    for entry in memory_map.entries() {
        log::debug!("{entry:?}");
    }
    log::debug!("Entries count: {}", memory_map.len());
    logger().flush();
    unsafe { memory::init(memory_map) };
    log::debug!("Initialized memory");
    logger().flush();

    interrupts::init();
    log::debug!("Initialized interrupts");
    logger().flush();
    int3();

    unsafe { x86_64_init::init(OFFSET_MAP_VIRT_ADDR, rsdp) };

    if let Some(serial) = serial::init() {
        log::info!("Switching logging to serial");
        logger().flush();
        let frame_buffer = logger::switch_to_serial(serial);
        log::info!("Switched logging to serial");
        if let Some(frame_buffer) = frame_buffer {
            // The frame buffer box was allocated by UEFI and so we can't free it
            core::mem::forget(frame_buffer);
        }
    }

    x86_64::instructions::interrupts::enable();
    log::info!("Enabled interrupts");
    logger().flush();

    execute_future(async {
        let mut count = 0_u64;
        loop {
            log::info!("Count: {count}");
            logger().flush();
            sleep(Duration::from_secs(1)).await;
            count += 1;
        }
    });

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
