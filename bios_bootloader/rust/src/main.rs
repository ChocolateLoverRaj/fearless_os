#![no_std]
#![no_main]
mod logger;

use core::{arch::naked_asm, panic::PanicInfo};

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

type Int10 = extern "C" fn(u8);
type Int15 = unsafe extern "C" fn(u16, u16);

#[repr(C)]
struct BootloaderTable {
    int_10: u16,
    int_15: u16,
}

unsafe extern "C" fn rust_start(_: u64, _: u64, bootloader_table: &BootloaderTable, _: u64) -> ! {
    let int_10 = unsafe { core::mem::transmute::<_, Int10>(bootloader_table.int_10 as usize) };
    logger::init(int_10);
    log::info!("Hello from Rust in long mode!");
    loop {}
}

#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    let _ = panic_info;
    loop {}
}
