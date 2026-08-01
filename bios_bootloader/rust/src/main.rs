#![no_std]
#![no_main]
mod bios;
mod logger;
mod writer_with_cr;

use core::{arch::naked_asm, panic::PanicInfo};

use arbitrary_int::u9;
use zerocopy::FromZeros;

use crate::bios::{
    BootloaderTable,
    vesa::{ModeInfo, VesaModeAttributes, VideoModesIterator},
};

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
    log::debug!("sector 0: {buffer:x?}");
    let mut info = FromZeros::new_zeroed();
    let info = bootloader_table
        .vesa_get_controller_info
        .call(&mut info)
        .unwrap();
    log::debug!("VESA info: {info:#X?} {info:p}");
    log::info!(
        "VESA version: {:#X?}. Ptr: {:#X}:{:#X}",
        info.vbe_version,
        info.video_modes_segment,
        info.video_modes_offset
    );
    if info.vbe_version != [0x00, 0x03] {
        panic!("Unsupported VESA version: {:#X?}", info.vbe_version);
    }
    let video_modes = unsafe { VideoModesIterator::new(&info) };

    let mut best_mode = None::<(u16, ModeInfo)>;
    for video_mode in video_modes {
        let mut buffer = [Default::default(); _];
        let info = bootloader_table
            .vesa_get_mode_info
            .call(video_mode, &mut buffer)
            .unwrap();
        log::debug!("Video mode: {video_mode:#X}: {info:#X?}");
        let a = VesaModeAttributes::new_with_raw_value(info.mode_attributs.get());
        if a.mode_type() && a.linear_frame_buffer_mode_available() {
            let width = info.x_resolution.get();
            let height = info.y_resolution.get();
            let bpp = info.bits_per_pixel;
            log::debug!("Graphics mode: {width}x{height}, {bpp}-bit color");
            if best_mode.is_none_or(|(_, best_info)| {
                width >= best_info.x_resolution.get()
                    && height >= best_info.y_resolution.get()
                    && bpp >= best_info.bits_per_pixel
            }) {
                best_mode = Some((video_mode, *info));
            }
        }
    }
    if let Some((mode, info)) = best_mode {
        let width = info.x_resolution.get();
        let height = info.y_resolution.get();
        let bpp = info.bits_per_pixel;
        let ptr = info.phys_base_ptr.get();
        log::info!("Best mode: {width}x{height}, {bpp}-bit color, {ptr:#X}");
        bootloader_table
            .vesa_set_mode
            .call(u9::new(mode), true)
            .unwrap();
        // let ptr = usize::try_from(info.phys_base_ptr.get()).unwrap() as *mut u8;
        // let bytes = usize::try_from(info.lin_bytes_per_scan.get()).unwrap()
        //     * usize::try_from(height).unwrap()
        //     * usize::try_from(bpp).unwrap();
        // unsafe { ptr.write_bytes(0x67, bytes) };

        // log::info!("Set mode");
    } else {
        log::info!("No suitable graphics mode found");
    }
    loop {}
}

#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    log::error!("{panic_info}");
    log::error!("---");
    loop {}
}
