#![no_std]
#![no_main]
use core::{
    arch::naked_asm,
    cmp::{max, min},
    panic::PanicInfo,
    ptr::{NonNull, addr_of},
};

use common::{
    BIG_STAGE_LOAD_ADDR, OFFSET_MAP_VIRT_ADDR, SECTOR_1,
    big_stage_api::{self, BigStageEntryInfo},
    bios::BiosFns,
    logger,
    paging::{
        LeafMapping, LeafMappingFlags, LeafMappingSize, PageTable, ScratchPageTable, TopLevel,
        TopLevelPageTable,
    },
    pat::WRITE_BACK_INDEX,
};
use spin::Once;
use x86_64::{instructions::hlt, registers::control::Cr3};

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

#[repr(C)]
struct RustStartRet {
    jmp_addr: big_stage_api::Entry,
    info: &'static BigStageEntryInfo,
}

unsafe extern "C" fn rust_start(_: usize, partition_start_lba: u64, dl: u8) -> RustStartRet {
    static BIOS_FNS: Once<BiosFns> = Once::new();
    let bios_fns = BIOS_FNS.call_once(|| unsafe { BiosFns::new(None) });
    logger::init(&bios_fns);
    log::info!("Hello from small Rust. DL={dl:#X}. Partition start LBA: {partition_start_lba:#X}.");
    for m in MemoryIterator::default() {
        let m = m.unwrap();
        log::info!("Memory entry: {:#X?}", m);
    }

    let next_stage_file_len = env!("NEXT_STAGE_FILE_LEN").parse::<u64>().unwrap();
    let big_stage_mem_len = env!("NEXT_STAGE_MEM_LEN").parse::<u64>().unwrap();
    // Actual virtual address to jump to
    let next_stage_jmp_addr = env!("NEXT_STAGE_JMP_ADDR").parse::<u64>().unwrap();
    log::info!(
        "next next_stage_file_len: {next_stage_file_len:#X}, next_stage_mem_len: {big_stage_mem_len:#X}, next_stage_jmp_addr: {next_stage_jmp_addr:#X}"
    );

    // We use 2M because it saves memory compared to 4K
    // We don't use 1G because then we would need to find a 1G aligned phys addr and our target computer (Lenovo Z560) doesn't support 1G pages.
    // I think many BIOS laptops also don't support 1G pages.
    let big_stage_page_size = LeafMappingSize::_2M;

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
        m.start = m.start.next_multiple_of(big_stage_page_size.byte_size());
        if m.is_empty() {
            continue;
        }
        let file_mem_needed_end = (m.start + next_stage_file_len).next_multiple_of(512);
        if m.end < file_mem_needed_end {
            continue;
        }
        let mem_mem_needed_end = m.start + big_stage_mem_len;
        if m.end < mem_mem_needed_end {
            continue;
        }
        next_stage_phys_addr = Some(m.start);
    }
    let read_buffer_addr = read_buffer_addr.expect("not enough free low mem for read buffer");
    let big_stage_phys_addr = next_stage_phys_addr.expect("no contiguous phys mem for next stage");
    log::info!(
        "read_buffer_addr: {read_buffer_addr:#X}, next_stage_phys_addr: {big_stage_phys_addr:#X}"
    );

    let top_level = TopLevel::Maps256T;
    let page_table_128_t = Cr3::read().0.start_address().as_u64();
    let top_level_table_phys_addr = page_table_128_t;
    // Safety: we and the page table sare identity mapped, the page table is valid
    let mut pt = unsafe { TopLevelPageTable::new(0, top_level_table_phys_addr, top_level) };

    let mapping_flags = LeafMappingFlags {
        writable: true,
        user_mode_accessible: false,
        executable: true,
        pat_index: WRITE_BACK_INDEX,
    };

    // Up to 2 pages to offset map fist 1 GiB
    // Up to 4 pages to offset map kernel (it may be on a border)
    // Up to 2 pages to map the kernel to its static load address
    static SCRATCH_TABLES: [PageTable; 10] = [PageTable::new(); _];
    let mut scratch_tables = SCRATCH_TABLES
        .iter()
        .map(|page_table| unsafe { ScratchPageTable::new(addr_of!(*page_table) as u64) });

    // Offset map the first 1 GiB
    let offset_mapping_size = LeafMappingSize::max_supported();
    let first_page = OFFSET_MAP_VIRT_ADDR;
    let first_frame = 0;
    let mappings_count = 0x40000000 / offset_mapping_size.byte_size();
    for i in 0..mappings_count {
        let mapping = LeafMapping::new(
            offset_mapping_size,
            first_page + i * offset_mapping_size.byte_size(),
            first_frame + i * offset_mapping_size.byte_size(),
            mapping_flags,
        );
        log::debug!("first 1 GiB mapping: {mapping:X?}.");
        unsafe { pt.ensure_mapped_leaf(mapping, &mut scratch_tables) }.unwrap();
    }
    // Offset map the big stage
    let first_page_phys_addr =
        big_stage_phys_addr / offset_mapping_size.byte_size() * offset_mapping_size.byte_size();
    let first_page_virt_addr = OFFSET_MAP_VIRT_ADDR + first_page_phys_addr;
    let last_page_exclusive_phys_addr =
        (big_stage_phys_addr + big_stage_mem_len).next_multiple_of(offset_mapping_size.byte_size());
    let mappings_count =
        (last_page_exclusive_phys_addr - first_page_phys_addr) / offset_mapping_size.byte_size();
    for i in 0..mappings_count {
        let mapping = LeafMapping::new(
            offset_mapping_size,
            first_page_virt_addr + i * offset_mapping_size.byte_size(),
            first_page_phys_addr + i * offset_mapping_size.byte_size(),
            mapping_flags,
        );
        log::debug!("big stage mapping {mapping:X?}.");
        unsafe { pt.ensure_mapped_leaf(mapping, &mut scratch_tables) }.unwrap();
    }

    // Map the big stage at it's static load address
    let first_page = BIG_STAGE_LOAD_ADDR;
    let first_frame = big_stage_phys_addr;
    let mappings_count = big_stage_mem_len.div_ceil(big_stage_page_size.byte_size());
    for i in 0..mappings_count {
        unsafe {
            pt.ensure_mapped_leaf(
                LeafMapping::new(
                    big_stage_page_size,
                    first_page + (i * big_stage_page_size.byte_size()),
                    first_frame + (i * big_stage_page_size.byte_size()),
                    mapping_flags,
                ),
                &mut scratch_tables,
            )
        }
        .unwrap()
    }

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
    log::info!("Big stage starting LBA: {starting_lba:#X}.");
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
        let src = NonNull::new(read_buffer_addr as *mut u8).unwrap();
        let dest = unsafe {
            NonNull::new_unchecked((BIG_STAGE_LOAD_ADDR + sectors_copied * 512) as *mut u8)
        };
        unsafe {
            src.copy_to_nonoverlapping(
                dest,
                (sectors_to_copy_this_iteration * 512).try_into().unwrap(),
            )
        };

        sectors_copied += sectors_to_copy_this_iteration;
    }

    log::info!("Jumping to big stage ({next_stage_jmp_addr:#X}");
    let f = unsafe {
        core::mem::transmute::<_, big_stage_api::Entry>(next_stage_jmp_addr as *const ())
    };
    let big_stage_entry_info = BigStageEntryInfo {
        low_used_mem_len: low_used_len,
        big_stage_phys_start: big_stage_phys_addr,
    };
    log::info!("Passing info to next stage: {big_stage_entry_info:#X?}");
    static ENTRY_INFO: Once<BigStageEntryInfo> = Once::new();
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
