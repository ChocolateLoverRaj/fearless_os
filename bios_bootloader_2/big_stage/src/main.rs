#![no_std]
#![no_main]
#![feature(abi_x86_interrupt, allocator_api)]
extern crate alloc;

mod acpi_events;
mod acpi_handler;
mod apic;
mod bios_data_area;
mod free_iterator;
mod global_allocator;
mod initial_pmm;
mod interrupts;
mod linked_list;
mod memory;
mod pat;
mod physical_memory;
mod range_utils;
mod scratch_tables;
mod vmm;

use core::{
    arch::naked_asm,
    num::NonZero,
    panic::PanicInfo,
    ptr::NonNull,
    sync::atomic::{AtomicU16, Ordering},
};

use acpi::{AcpiTables, platform::AcpiPlatform, rsdp::Rsdp, sdt::fadt::Fadt};
use arbitrary_int::u9;
use common::{
    big_stage_api::BigStageEntryInfo,
    bios::{
        BiosFns,
        vesa::{ModeInfo, VesaModeAttributes},
    },
    logger,
};
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

const STACK_SIZE: usize = 0x80000;
#[repr(C, align(16))]
struct Stack {
    data: [u8; STACK_SIZE],
}
static mut STACK: Stack = Stack { data: [0; _] };

const DYNAMIC_VIRT_ADDR: u64 = 0xFFFF_8000_4000_0000;
const DYNAMIC_VIRT_LEN: u64 = 0x3FFFC0000000;

struct UsableMemNode {
    start: u64,
    len: u64,
    used_len: u64,
}

static BIOS_FNS: Once<BiosFns> = Once::new();

unsafe extern "C" fn rust_start(info: &BigStageEntryInfo) -> ! {
    // Safety: BIOS fns are still mapped and the old real-mode stack is completely free for us to use
    let bios_fns = *BIOS_FNS.call_once(|| unsafe {
        BiosFns::new(Some(
            NonZero::new(ORIGINAL_STACK_POINTER.load(Ordering::Relaxed)).unwrap(),
        ))
    });
    logger::init(bios_fns);
    log::info!("Hello from big stage. {info:#X?}.");
    unsafe { memory::init(info, bios_fns) };
    interrupts::init();
    log::info!("initialized interrupts.");

    int3();

    let vbe_info = bios_fns.get_vbe_info().unwrap();
    // log::info!("VBE: {vbe_info:#X?}");
    let vbe_version = vbe_info.info().vbe_version;
    log::info!("VBE version: {vbe_version:#X?}.");
    if vbe_version != [0x00, 0x03] {
        panic!("Unsupported VESA version: {vbe_version:#X?}.",);
    }

    let mut best_mode = None::<(u9, ModeInfo)>;
    for video_mode in vbe_info.mode_list() {
        let info = bios_fns.vesa_get_mode_info(video_mode).unwrap();
        log::debug!("Video mode: {video_mode:#X}: {info:#X?}");
        let attributes = VesaModeAttributes::new_with_raw_value(info.mode_attributs.get());
        if attributes.mode_type() && attributes.linear_frame_buffer_mode_available() {
            let width = info.x_resolution.get();
            let height = info.y_resolution.get();
            let bpp = info.bits_per_pixel;
            log::debug!("Graphics mode: {width}x{height}, {bpp}-bit color");
            if best_mode.is_none_or(|(_, best_info)| {
                width >= best_info.x_resolution.get()
                    && height >= best_info.y_resolution.get()
                    && bpp >= best_info.bits_per_pixel
            }) {
                best_mode = Some((video_mode, info));
            }
        }
    }
    if let Some((mode, info)) = best_mode {
        let width = info.x_resolution.get();
        let height = info.y_resolution.get();
        let bpp = info.bits_per_pixel;
        let ptr = info.phys_base_ptr.get();
        log::info!("Best mode: {mode}, {width}x{height}, {bpp}-bit color, {ptr:#X}");
        // bootloader_table
        //     .vesa_set_mode
        //     .call(u9::new(mode), true)
        //     .unwrap();
        // let ptr = usize::try_from(info.phys_base_ptr.get()).unwrap() as *mut u8;
        // let bytes = usize::try_from(info.lin_bytes_per_scan.get()).unwrap()
        //     * usize::try_from(height).unwrap()
        //     * usize::try_from(bpp).unwrap();
        // unsafe { ptr.write_bytes(0x67, bytes) };

        // log::info!("Set mode");
    } else {
        log::info!("No suitable graphics mode found");
    }

    log::info!("searching for bios data area");
    let bios_data_area = unsafe { NonNull::new(0x400 as *mut BiosDataArea).unwrap().as_ref() };
    log::trace!("bios_data_area: {:#X?}", bios_data_area);

    let rsdp = unsafe { Rsdp::search_for_on_bios(AcpiHandler {}) }.unwrap();
    log::info!("RSDP: {:#X?}", rsdp.get());
    let acpi_tables =
        unsafe { AcpiTables::from_rsdp(AcpiHandler {}, rsdp.physical_start) }.unwrap();
    for (_phys_addr, table) in acpi_tables.table_headers() {
        let signature = table.signature;
        log::info!("ACPI Table: {signature}.");
    }
    acpi_handler::init(&acpi_tables);

    let platform = AcpiPlatform::new(acpi_tables, AcpiHandler {}).unwrap();
    log::info!("Got platform");

    unsafe { apic::init(&platform) };

    platform.enter_acpi_mode().unwrap();
    log::info!("Entered ACPI mode");

    let fadt = platform.tables.find_table::<Fadt>().unwrap();
    let sci_interrupt = fadt.sci_interrupt;
    log::info!("SCI Interrupt IRQ: {sci_interrupt:#X}");
    unsafe { acpi_events::init(platform) };

    x86_64::instructions::interrupts::enable();
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
