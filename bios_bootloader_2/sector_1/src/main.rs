#![no_std]
#![no_main]
mod bios;
mod logger;
mod writer_with_cr;

use core::{
    arch::naked_asm,
    cmp::{max, min},
    panic::PanicInfo,
    ptr::addr_of,
};

use x86_64::{instructions::hlt, structures::gdt::GlobalDescriptorTable};

use crate::bios::MemoryIterator;

unsafe extern "C" {
    static __bss_start: *const u8;
    static __bss_u64s_to_copy: *const u8;
    static __bss_end: *const u8;
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

    let next_stage_file_len = env!("NEXT_STAGE_FILE_LEN").parse::<u64>().unwrap();
    let next_stage_mem_len = env!("NEXT_STAGE_MEM_LEN").parse::<u64>().unwrap();
    // Actual virtual address to jump to
    let next_stage_jmp_addr = env!("NEXT_STAGE_JMP_ADDR").parse::<u64>().unwrap();
    log::info!(
        "next next_stage_file_len: {next_stage_file_len:#X}, next_stage_mem_len: {next_stage_mem_len:#X}, next_stage_jmp_addr: {next_stage_jmp_addr:#X}"
    );

    // Find a 512 B * 127 buffer in real-mode accessible memory that we can use as a bounce buffer for reading the next stage
    #[derive(Debug)]
    struct UsableMem {
        start: u64,
        len: u64,
    }
    let self_stage_end = u64::try_from(addr_of!(__bss_end).addr()).unwrap();
    let mut low_used_len = self_stage_end;
    let low_mem_end: u64 = 0xFFFF * 16 + 0x10000;
    let buffer_len = 512 * 127;
    let mut read_buffer_addr = None;
    let mut next_stage_phys_addr = None;
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
        if read_buffer_addr.is_some() && next_stage_phys_addr.is_some() {
            break;
        }
        log::info!("usable mem: {m:X?}");
        if read_buffer_addr.is_none() && m.start < low_mem_end {
            let mut low_free = max(m.start, low_used_len)..min(m.start + m.len, low_mem_end);
            low_free.start = low_free.start.next_multiple_of(16);
            if low_free.end - low_free.start >= buffer_len {
                low_used_len = low_free.start + buffer_len;
                read_buffer_addr = Some(low_free.start);
            }
        }
        let mut m = m.start..m.start + m.len;
        m.start = max(m.start, low_used_len);
        // We're loading the next stage
        m.start = m.start.next_multiple_of(16);
        if m.is_empty() {
            continue;
        }
        // See if we can fit the next stage
        // The file part will involve copying entire blocks of 512 B
        // The mem part doesn't need to end block aligned
        m.start = m.start.next_multiple_of(512);
        if m.is_empty() {
            continue;
        }
        let file_mem_needed_end = (m.start + next_stage_file_len).next_multiple_of(512);
        if m.end < file_mem_needed_end {
            continue;
        }
        let mem_mem_needed_end = m.start + next_stage_mem_len;
        if m.end < mem_mem_needed_end {
            continue;
        }
        next_stage_phys_addr = Some(m.start);
    }
    let read_buffer_addr = read_buffer_addr.expect("not enough free low mem for read buffer");
    let next_stage_phys_addr = next_stage_phys_addr.expect("no contiguous phys mem for next stage");
    log::info!(
        "read_buffer_addr: {read_buffer_addr:#X}, next_stage_phys_addr: {next_stage_phys_addr:#X}"
    );

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
