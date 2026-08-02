#![no_std]
#![no_main]

use core::{arch::naked_asm, panic::PanicInfo};

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

unsafe extern "C" fn rust_start() -> ! {
    for c in b"HEllo from Rust\r\nHello again!" {
        int_10(*c);
    }
    loop {}
}

#[unsafe(naked)]
extern "C" fn int_10(char: u8) {
    naked_asm!(
        "
        push 0x10
        push 0x9000
        retfq
        "
    )
}

#[panic_handler]
fn panic_handler(_panic_info: &PanicInfo) -> ! {
    loop {}
}
