#![no_std]
#![no_main]
#![feature(abi_x86_interrupt, allocator_api)]
extern crate alloc;

mod acpi_events;
mod acpi_handler;
mod apic;
mod bios_data_area;
mod config;
mod ehci;
mod frame_buffer;
mod frame_buffer_embedded_graphics;
mod frame_buffer_info;
mod frame_buffer_log_target;
mod frame_buffer_writer;
mod free_iterator;
mod global_allocator;
mod initial_pmm;
mod interrupts;
mod linked_list;
mod log_target;
mod logger;
mod memory;
mod pat;
mod physical_memory;
mod range_utils;
mod rgb_pixel_info;
mod scratch_tables;
mod uart_log_target;
mod vesa_text_log_target;
mod vmm;

use core::{
    arch::naked_asm,
    num::NonZero,
    panic::PanicInfo,
    ptr::NonNull,
    str::FromStr,
    sync::atomic::{AtomicU16, Ordering},
};

use acpi::{
    AcpiTables, HpetInfo,
    aml::{
        self,
        namespace::AmlName,
        pci_routing::{PciRoutingTable, Pin},
    },
    platform::AcpiPlatform,
    rsdp::Rsdp,
    sdt::{fadt::Fadt, mcfg::Mcfg},
};
use alloc::vec;
use arbitrary_int::{traits::Integer, u3, u5};
use common::{
    big_stage_api::BigStageEntryInfo,
    bios::BiosFns,
    paging::LeafMappingFlags,
    pat::{STRONG_UNCACHEABLE_INDEX, WRITE_THROUGH_INDEX},
};
use ez_ehci::{
    AnyEhci, InitDeviceBuffer, MappedMem, PCI_CLASS, PCI_PROG_IF, PCI_SUBCLASS, PeriodicFrameList,
    RunOutput, TryTakeOutput, new_ehci,
};
use ez_hpet::{HPET_MMIO_SIZE, Hpet};
use ez_pci::{BarWithSize, MemoryBarAddrAndSizeU64, PciAccess, PciFunction};
use log::logger;
use spin::Once;
use uart_16550::Uart16550Tty;
use x86_64::instructions::{hlt, interrupts::int3};

use crate::{
    acpi_handler::{AcpiHandler, PCIE_MAPPINGS, SEGMENT_MAPPED_LEN},
    bios_data_area::BiosDataArea,
    config::CONFIG,
    memory::{alloc_phys, map_phys},
};

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

    log::info!("searching for bios data area");
    let bios_data_area = unsafe { NonNull::new(0x400 as *mut BiosDataArea).unwrap().as_ref() };
    log::trace!("bios_data_area: {:#X?}", bios_data_area);
    let io_ports = bios_data_area.io_ports_com;
    log::info!("io_ports: {io_ports:#X?}");
    if let Some(io_port) = io_ports.iter().find_map(|port| NonZero::new(port.get())) {
        log::info!("Using COM port {io_port:#X}.");
        logger::init_uart(
            unsafe { Uart16550Tty::new_port(io_port.get(), Default::default()) }.unwrap(),
        );
        log::info!("Set logging to UART");
    }

    frame_buffer::init(bios_fns);

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

    if CONFIG.enter_acpi_mode {
        platform.enter_acpi_mode().unwrap();
        log::info!("Entered ACPI mode");
    }

    let fadt = platform.tables.find_table::<Fadt>().unwrap();
    let sci_interrupt = fadt.sci_interrupt;
    log::info!("SCI Interrupt IRQ: {sci_interrupt:#X}");

    unsafe { acpi_events::init(platform) };

    ehci::run()
}

#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    log::error!("{panic_info}");
    logger().flush();
    loop {
        hlt();
    }
}
