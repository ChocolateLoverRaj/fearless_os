#![no_std]
#![no_main]
mod bios;
mod logger;
mod writer_with_cr;

use core::{arch::naked_asm, panic::PanicInfo};

use x86_64::instructions::hlt;

use crate::bios::MemoryIterator;

unsafe extern "C" {
    static __bss_start: *const u8;
    static __bss_u64s_to_copy: *const u8;
}

#[unsafe(naked)]
#[unsafe(link_section = ".text._start")]
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

unsafe extern "C" fn rust_start(_: usize, _: usize, dl: u8) -> ! {
    logger::init();
    log::info!("Hello from small Rust. DL={dl:#X}.");
    for m in MemoryIterator::default() {
        let m = m.unwrap();
        log::info!("Memory entry: {:#X?}", m);
    }
    loop {
        hlt();
    }
}

#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    log::error!("{panic_info}");
    loop {
        hlt();
    }
}
