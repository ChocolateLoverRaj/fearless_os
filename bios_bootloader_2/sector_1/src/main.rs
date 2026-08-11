#![no_std]
#![no_main]
use core::{
    arch::naked_asm,
    cmp::{max, min},
    iter,
    panic::PanicInfo,
    ptr::{NonNull, addr_of},
};

use common::{
    BIG_STAGE_ADDR, SECTOR_1,
    big_stage_api::{self, BigStageEntryInfo},
    bios::BiosFns,
    logger,
    paging::{
        self, LeafMapping, LeafMappingSize, MapError, PageTable, ScratchPageTable,
        TableMappingSize, TableMappingVirtAddr, TopLevel, TopLevelPageTable,
    },
};
use spin::Once;
use x86_64::{
    PhysAddr, VirtAddr,
    instructions::hlt,
    registers::control::Cr3,
    structures::{DescriptorTablePointer, gdt::GlobalDescriptorTable},
};

use common::bios::{MemoryIterator, RealModeAddr, extended_read};

unsafe extern "C" {
    static __bss_start: *const u8;
    static __bss_u64s_to_copy: *const u8;
    static __data_end: *const u8;
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

        call {rust_start}
        mov rdi, rdx
        call rax
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
static GDT_PTR: Once<DescriptorTablePointer> = Once::new();

static BIOS_FNS: Once<BiosFns> = Once::new();
static PAGE_TABLES_MEM: [PageTable; 2] = [PageTable::new(), PageTable::new()];
static ENTRY_INFO: Once<BigStageEntryInfo> = Once::new();

#[repr(C)]
struct RustStartRet {
    jmp_addr: big_stage_api::Entry,
    info: &'static BigStageEntryInfo,
}

unsafe extern "C" fn rust_start(_: usize, partition_start_lba: u64, dl: u8) -> RustStartRet {
    GDT.load();
    let bios_fns = BIOS_FNS.call_once(|| unsafe { BiosFns::new(None) });
    logger::init(&bios_fns);
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
    let mut low_used_len = u64::try_from(addr_of!(__bss_end).addr()).unwrap();
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

        // See if we can fit the next stage
        // The file part will involve copying entire blocks of 512 B
        // The mem part doesn't need to end block aligned
        m.start = m.start.next_multiple_of(0x200000);
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

    // let top_level = TopLevel::max_supported();
    let top_level = TopLevel::Maps256T;
    // static PAGE_TABLE_128_P: paging::PageTable = paging::PageTable::new();
    let page_table_128_t = Cr3::read().0.start_address().as_u64();
    // let top_level_table_phys_addr = match top_level {
    //     TopLevel::Maps128P => addr_of!(PAGE_TABLE_128_P) as u64,
    //     TopLevel::Maps256T => page_table_128_t,
    // };
    let top_level_table_phys_addr = page_table_128_t;
    // Safety: we and the page table sare identity mapped, the page table is valid
    let mut pt = unsafe { TopLevelPageTable::new(0, top_level_table_phys_addr, top_level) };
    // if top_level == TopLevel::Maps128P {
    //     log::info!("Creating 5-level page tables!");
    //     unsafe {
    //         pt.attach_existing_page_table(
    //             TableMappingVirtAddr::new(0, TableMappingSize::_256T),
    //             page_table_128_t,
    //             iter::empty(),
    //         )
    //     }
    //     .unwrap();
    // }

    let read_buffer_real_addr =
        RealModeAddr::try_from(u32::try_from(read_buffer_addr).unwrap()).unwrap();

    let total_sectors_to_copy = next_stage_file_len.next_multiple_of(512) / 512;
    let starting_lba = partition_start_lba
        + 1
        + (u64::try_from(addr_of!(__data_end).addr())
            .unwrap()
            .next_multiple_of(512)
            - u64::from(SECTOR_1))
            / 512;
    log::info!("starting LBA: {starting_lba:#X}");
    let mut sectors_copied = 0;
    while sectors_copied < total_sectors_to_copy {
        let sectors_to_copy_this_iteration = min(total_sectors_to_copy - sectors_copied, 127);
        unsafe {
            extended_read(
                dl,
                starting_lba + sectors_copied,
                read_buffer_real_addr,
                sectors_to_copy_this_iteration.try_into().unwrap(),
            )
        }
        .unwrap();

        // Worst case is that the max 127 sectors cross a 2 MiB boundary
        // We can ensure that the first and last page in the range are mapped
        let first_page = (BIG_STAGE_ADDR + sectors_copied * 512) / LeafMappingSize::_2M.byte_size()
            * LeafMappingSize::_2M.byte_size();
        let first_frame = (next_stage_phys_addr + sectors_copied * 512)
            / LeafMappingSize::_2M.byte_size()
            * LeafMappingSize::_2M.byte_size();
        let last_page = (BIG_STAGE_ADDR
            + (sectors_copied + sectors_to_copy_this_iteration - 1) * 512)
            / LeafMappingSize::_2M.byte_size()
            * LeafMappingSize::_2M.byte_size();
        let last_frame = (next_stage_phys_addr
            + (sectors_copied + sectors_to_copy_this_iteration - 1) * 512)
            / LeafMappingSize::_2M.byte_size()
            * LeafMappingSize::_2M.byte_size();

        let mut i = PAGE_TABLES_MEM
            .iter()
            .map(|page_table| unsafe { ScratchPageTable::new(addr_of!(*page_table) as u64) });
        for (page, frame) in [(first_page, first_frame), (last_page, last_frame)] {
            log::info!("Mapping {page:#X?} to {frame:#X?}");
            let result = unsafe {
                pt.map_leaf(
                    LeafMapping::new(paging::LeafMappingSize::_2M, page, frame),
                    &mut i,
                )
            };
            match result {
                Ok(_)
                | Err(MapError::AlreadyMapped {
                    table: _,
                    entry_index: _,
                }) => {}
                Err(e) => panic!("{:?}", e),
            }
        }

        // #[repr(C, align(512))]
        // struct Block {
        //     bytes: [u8; 512],
        // }
        let src = NonNull::new(read_buffer_addr as *mut u8).unwrap();
        let dest =
            unsafe { NonNull::new_unchecked((BIG_STAGE_ADDR + sectors_copied * 512) as *mut u8) };
        log::info!("copying...");
        unsafe {
            src.copy_to_nonoverlapping(
                dest,
                (sectors_to_copy_this_iteration * 512).try_into().unwrap(),
            )
        };

        sectors_copied += sectors_to_copy_this_iteration;
    }

    let next_stage_file = unsafe {
        core::slice::from_raw_parts(BIG_STAGE_ADDR as *const u8, next_stage_file_len as usize)
    };
    let crc = crc32fast::hash(next_stage_file);
    log::info!(
        "crc32 of file: {crc:X?}. file len: {}",
        next_stage_file.len()
    );
    let next_stage_mem = unsafe {
        core::slice::from_raw_parts(BIG_STAGE_ADDR as *const u8, next_stage_mem_len as usize)
    };
    let crc32 = crc32fast::hash(next_stage_mem);
    log::info!(
        "crc32 of mem: {crc32:X?}. mem len: {}",
        next_stage_mem.len()
    );
    log::info!("Jumping to big stage ({next_stage_jmp_addr:#X}");
    let f = unsafe {
        core::mem::transmute::<_, big_stage_api::Entry>(next_stage_jmp_addr as *const ())
    };
    let gdt_ptr = GDT_PTR.call_once(|| DescriptorTablePointer {
        limit: GDT.limit(),
        base: VirtAddr::from_ptr(GDT.entries().as_ptr()),
    });
    let big_stage_entry_info = BigStageEntryInfo {
        low_used_mem_len: low_used_len,
        big_stage_phys_start: next_stage_phys_addr,
        low_mem_gdt_ptr_addr: core::ptr::from_ref(gdt_ptr) as u64,
    };
    log::info!("Passing info to next stage: {big_stage_entry_info:#X?}");
    let info = ENTRY_INFO.call_once(|| big_stage_entry_info);
    RustStartRet { jmp_addr: f, info }
    // unsafe { asm!("call {}", in(reg) next_stage_jmp_addr, options(noreturn)) };
}

#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    log::error!("{panic_info}");
    loop {
        hlt();
    }
}
