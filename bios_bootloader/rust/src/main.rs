#![no_std]
#![no_main]
use core::{arch::naked_asm, panic::PanicInfo};

unsafe extern "C" {
    static __bss_start: *const u8;
    static __bss_u64s_to_copy: *const u8;
}

static A: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

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

unsafe extern "C" fn rust_start(_: u64, _: u64, callback: extern "C" fn(u8)) -> ! {
    for c in "Hello from Rust in long mode!\r\n".as_bytes() {
        callback(*c);
    }
    A.store(true, core::sync::atomic::Ordering::Relaxed);
    loop {}
}

#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    let _ = panic_info;
    loop {}
}
