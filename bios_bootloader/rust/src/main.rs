#![no_std]
#![no_main]
mod bios;
mod logger;
mod writer_with_cr;

use core::{arch::naked_asm, panic::PanicInfo};

use zerocopy::FromZeros;

use crate::bios::{BootloaderTable, VideoModesIterator};

unsafe extern "C" {
    static __bss_start: *const u8;
    static __bss_u64s_to_copy: *const u8;
}

#[unsafe(naked)]
#[unsafe(link_section = ".text.start")]
#[unsafe(no_mangle)]
unsafe extern "C" fn _start() {
    naked_asm!(
        "
        // Zero the BSS
        xor rax, rax
        lea rdi, {__bss_start}
        lea rcx, {__bss_u64s_to_copy}
        rep stosq

        jmp {rust_start}
        ",
        __bss_start = sym __bss_start,
        __bss_u64s_to_copy = sym __bss_u64s_to_copy,
        rust_start = sym rust_start,
    )
}

unsafe extern "C" fn rust_start(_: u64, _: u64, bootloader_table: &BootloaderTable, _: u64) -> ! {
    logger::init(bootloader_table.int_10);
    log::info!("Hello from Rust in long mode!");
    let mut buffer = [Default::default(); _];
    let mut entry_index = 0;
    loop {
        let output = bootloader_table
            .int_15
            .call(&mut buffer, entry_index)
            .unwrap();
        log::info!("{output:#X?}");
        let Some(next_entry_index) = output.next_entry_index else {
            break;
        };
        entry_index = next_entry_index.get();
    }
    log::info!("Done reading entries");
    let mut buffer = [Default::default(); 512];
    let disk = bootloader_table.disk;
    log::info!("Reading disk: {disk:#X}");
    bootloader_table
        .extended_read
        .call(bootloader_table.disk, 0, &mut buffer)
        .unwrap();
    log::info!("sector 0: {buffer:x?}");
    let mut info = FromZeros::new_zeroed();
    let info = bootloader_table
        .vesa_get_controller_info
        .call(&mut info)
        .unwrap();
    log::info!("VESA info: {info:#X?} {info:p}");
    log::info!("VESA version: {:#X?}", info.vbe_version);
    let video_modes = unsafe { VideoModesIterator::new(&info) };
    for video_mode in video_modes {
        log::info!("Video mode: {video_mode:#X}");
    }
    loop {}
}

#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    log::error!("{panic_info}");
    log::error!("---");
    loop {}
}
