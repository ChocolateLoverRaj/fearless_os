#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
extern crate alloc;

mod acpi_handler;
mod bios_data_area;
mod free_iterator;
mod interrupts;
mod linked_list;
mod memory;
mod physical_memory;
mod range_utils;
mod virtual_memory;

use core::{
    arch::naked_asm,
    num::NonZero,
    panic::PanicInfo,
    ptr::NonNull,
    sync::atomic::{AtomicU16, Ordering},
};

use acpi::{AcpiTables, rsdp::Rsdp};
use common::{big_stage_api::BigStageEntryInfo, bios::BiosFns, logger};
use spin::Once;
use x86_64::instructions::{hlt, interrupts::int3};

use crate::{acpi_handler::AcpiHandler, bios_data_area::BiosDataArea};

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
        lea rsp, {stack}
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

const DYNAMIC_VIRT: u64 = 0xFFFFA00000000000;

struct UsableMemNode {
    start: u64,
    len: u64,
    used_len: u64,
}

static BIOS_FNS: Once<BiosFns> = Once::new();

unsafe extern "C" fn rust_start(info: &BigStageEntryInfo) -> ! {
    // Safety: BIOS fns are still mapped and the old real-mode stack is completely free for us to use
    let bios_fns = BIOS_FNS.call_once(|| unsafe {
        BiosFns::new(Some(
            NonZero::new(ORIGINAL_STACK_POINTER.load(Ordering::Relaxed)).unwrap(),
        ))
    });
    logger::init(bios_fns);
    log::info!("Hello from big stage. {info:#X?}.");
    unsafe { memory::init(info) };
    interrupts::init();
    log::info!("initialized interrupts.");

    int3();

    let bios_data_area = unsafe { NonNull::new(0x400 as *mut BiosDataArea).unwrap().as_ref() };
    log::info!("bios_data_area: {:#X?}", bios_data_area);

    // let ebda_pointer = bios_data_area.ebda_base_addr.get() as *mut u8;
    // let ebda_len = 0xA0000 - bios_data_area.ebda_base_addr.get();
    // let ebda = unsafe { slice::from_raw_parts(ebda_pointer, ebda_len as usize) };
    // let possible_rsdp = &ebda[bios_data_area.ebda_base_addr.get().next_multiple_of(16) as usize..];
    // let rsdp = possible_rsdp
    //     .array_windows::<16>()
    //     .find(|bytes| if bytes[..8] == b"RSD PTR\0" {
    //         Rsdp::
    //     });
    let rsdp = unsafe { Rsdp::search_for_on_bios(AcpiHandler {}) }.unwrap();
    log::info!("RSDP: {:#X?}", rsdp.get());
    let acpi_tables =
        unsafe { AcpiTables::from_rsdp(AcpiHandler {}, rsdp.physical_start) }.unwrap();
    for (phys_addr, table) in acpi_tables.table_headers() {
        let signature = table.signature;
        log::info!("ACPI Table: {signature}.");
    }
    log::info!("Done listing ACPI tables");

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
