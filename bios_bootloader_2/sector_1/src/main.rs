#![no_std]
#![no_main]
mod bios;
mod logger;
mod writer_with_cr;

use core::{arch::naked_asm, panic::PanicInfo};

use x86_64::{instructions::hlt, structures::gdt::GlobalDescriptorTable};

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

static GDT: GlobalDescriptorTable = GlobalDescriptorTable::from_raw_entries(&[
    // Null segment (required)
    0x0000000000000000,
    // Code 64
    0x00209A0000000000,
    // Code 32
    0x00CF9A000000FFFF,
    // Code 16
    0x000F9A000000FFFF,
    // Data 64
    0x0000920000000000,
    // Data 32
    0x00CF92000000FFFF,
    // Data 16
    0x000092000000FFFF,
]);

unsafe extern "C" fn rust_start(_: usize, partition_start_lba: u64, dl: u8) -> ! {
    GDT.load();
    logger::init();
    log::info!("Hello from small Rust. DL={dl:#X}. Partition start LBA: {partition_start_lba:#X}.");
    for m in MemoryIterator::default() {
        let m = m.unwrap();
        log::info!("Memory entry: {:#X?}", m);
    }

    // Find a 512 B * 127 buffer in real-mode accessible memory that we can use as a bounce buffer for reading the next stage
    #[derive(Debug)]
    struct UsableMem {
        start: u64,
        len: u64,
    }
    // let mut low_mem = None;
    for m in MemoryIterator::default()
        .map(|m| m.unwrap())
        .filter_map(|m| {
            if m.is_usable() {
                Some(UsableMem {
                    start: m.base_addr,
                    len: m.len,
                })
            } else {
                None
            }
        })
    {
        log::info!("usable mem: {m:X?}");
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
