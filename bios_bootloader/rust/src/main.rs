#![no_std]
#![no_main]
use core::{arch::naked_asm, panic::PanicInfo};

unsafe extern "C" {
    static __stack_top: *const u8;
}

#[unsafe(naked)]
#[unsafe(link_section = ".text.start")]
unsafe extern "C" fn start() {
    naked_asm!(
        "
        mov rip, {stack_top}
        jmp {rust_start}
        ",
        stack_top = sym __stack_top,
        rust_start = sym rust_start,
    )
}

unsafe extern "C" fn rust_start() {
    loop {}
}

#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    let _ = panic_info;
    loop {}
}
