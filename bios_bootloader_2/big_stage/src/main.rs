#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
extern crate alloc;

mod allocator;
mod interrupts;

use core::{
    arch::naked_asm,
    cmp::min,
    mem::MaybeUninit,
    num::NonZero,
    panic::PanicInfo,
    ptr::addr_of,
    sync::atomic::{AtomicU16, Ordering},
};

use alloc::{
    boxed::Box,
    vec::{self, Vec},
};
use bitmap_allocator::{BitAlloc, BitAlloc1M};
use common::{
    STACK_TOP,
    big_stage_api::BigStageEntryInfo,
    bios::{self, BiosFns, MemoryIterator},
    logger,
};
use spin::{Mutex, Once};
use x86_64::{
    VirtAddr,
    instructions::{hlt, interrupts::int3},
    structures::DescriptorTablePointer,
};

use crate::allocator::TALC;

unsafe extern "C" {
    static __start: *const u8;
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
        // Preserve rdi
        push rdi

        // Zero the BSS
        xor rax, rax
        lea rdi, {__bss_start}
        lea rcx, {__bss_u64s_to_copy}
        rep stosq

        pop rdi
        mov [{original_stack_pointer}], sp
        mov rsp, {stack}
        add rsp, {stack_size}
        call {rust_start}
        ",
        __bss_start = sym __bss_start,
        __bss_u64s_to_copy = sym __bss_u64s_to_copy,
        rust_start = sym rust_start,
        original_stack_pointer = sym ORIGINAL_STACK_POINTER,
        stack = sym STACK,
        stack_size = const STACK_SIZE,
    )
}

static ORIGINAL_STACK_POINTER: AtomicU16 = AtomicU16::new(0);

const STACK_SIZE: usize = 0x40000;
#[repr(C, align(16))]
struct Stack {
    data: [u8; STACK_SIZE],
}
static mut STACK: Stack = Stack { data: [0; _] };

const OFFSET_MAP: u64 = 0xFFFFC00000000000;

const DYNAMIC_VIRT: u64 = 0xFFFFA00000000000;

struct UsableMemNode {
    start: u64,
    len: u64,
    used_len: u64,
}

struct StaticStuff {
    /// Enough to store up to 4 GiB point.
    /// Each bit represents a 4 KiB phys frame starting a phys addr 0.
    free_phys_mem: BitAlloc1M,
    /// Each bit represents a 4 KiB page starting at virt addr [`DYNAMIC_VIRT`].
    free_virt_mem: BitAlloc1M,
}

static FREE_PHYS_MEM: Mutex<StaticStuff> = Mutex::new(StaticStuff {
    free_phys_mem: BitAlloc1M::DEFAULT,
    free_virt_mem: BitAlloc1M::DEFAULT,
});
static BIOS_FNS: Once<BiosFns> = Once::new();

unsafe extern "C" fn rust_start(info: &BigStageEntryInfo) -> ! {
    // Safety: BIOS fns are still mapped and the old real-mode stack is completely free for us to use
    let bios_fns = BIOS_FNS.call_once(|| unsafe {
        BiosFns::new(
            Some(NonZero::new(ORIGINAL_STACK_POINTER.load(Ordering::Relaxed)).unwrap()),
            Some(NonZero::new(info.low_mem_gdt_ptr_addr).unwrap()),
        )
    });
    logger::init(bios_fns);
    log::info!("Hello from big stage. {info:#X?}.");
    interrupts::init();
    log::info!("initialized interrupts.");

    let mut s = FREE_PHYS_MEM.lock();
    for a in MemoryIterator::default() {
        let a = a.unwrap();
        if a.is_usable() {
            let start = a.base_addr as usize;
            let end = start + a.len as usize;
            let start_page = start.next_multiple_of(0x1000) / 0x1000;
            let end_page = end / 0x1000;

            let range = start_page..min(end_page, BitAlloc1M::CAP);
            if !range.is_empty() {
                s.free_phys_mem.insert(range);
            }
        }
    }
    for used_range in [
        (0..info.low_used_mem_len),
        (info.big_stage_phys_start
            ..info.big_stage_phys_start
                + (addr_of!(__bss_end).addr() - addr_of!(__start).addr()) as u64),
    ] {
        let start_page = used_range.start / 0x1000;
        let end_page = used_range.end.next_multiple_of(0x1000) / 0x1000;
        let range = start_page as usize..end_page as usize;
        log::info!("removing range: {range:#X?}. Cap: {:#X}.", BitAlloc1M::CAP);
        s.free_phys_mem.remove(range);
    }

    s.free_virt_mem.insert(0..BitAlloc1M::CAP);

    let allocated_phys_mem = s.free_phys_mem.alloc().unwrap();
    let page = s.free_virt_mem.alloc().unwrap();
    log::info!("could alloc {page:#X}->{allocated_phys_mem:#X}.");

    int3();

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
