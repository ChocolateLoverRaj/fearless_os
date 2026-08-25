use core::{ops::Range, ptr::addr_of};

use common::{
    OFFSET_MAP_VIRT_ADDR,
    big_stage_api::BigStageEntryInfo,
    bios::MemoryIterator,
    paging::{LeafMapping, LeafMappingFlags, LeafMappingSize, MapError, TopLevelPageTable},
    pat::WRITE_BACK_INDEX,
};
use heapless::Vec;
use spin::{Mutex, Once};
use x86_64::registers::control::{Cr3, Cr4, Cr4Flags, Efer, EferFlags};

use crate::{
    __bss_end, __start,
    initial_pmm::InitialPmm,
    pat::{self},
    range_utils::{SubtractRangesIterator, is_overlap},
    scratch_tables::{InitialScratchTablesIterator, ScratchTablesIterator},
    vmm::Vmm,
};

static INITIAL_FREE_MEM: Once<Vec<Range<u64>, 32>> = Once::new();

struct Memory {
    pmm: InitialPmm<'static>,
    vmm: Vmm,
    pt: TopLevelPageTable,
}

static MEMORY: Once<Mutex<Memory>> = Once::new();

/// # Safety
///
/// Must be called exactly once.
pub unsafe fn init(info: &BigStageEntryInfo) {
    // Safety: doesn't break any existing mappings
    unsafe { pat::init() };

    // Enable no-execute flag
    unsafe { Efer::update(|efer| efer.insert(EferFlags::NO_EXECUTE_ENABLE)) };

    let mut mem_entries = MemoryIterator::default()
        .collect::<Result<heapless::Vec<_, 32>, _>>()
        .unwrap();

    // Make sure ranges are sorted
    mem_entries.sort_unstable_by(|a, b| a.base_addr.cmp(&b.base_addr));

    log::debug!("mem_entries: {mem_entries:#X?}");

    // Make sure ranges are not overlapping
    if is_overlap(
        mem_entries
            .iter()
            .map(|data| data.base_addr..data.base_addr + data.len),
    ) {
        panic!("overlap in mem entries");
    }

    let used_ranges = [
        (0..info.low_used_mem_len),
        (info.big_stage_phys_start
            ..info.big_stage_phys_start
                + (addr_of!(__bss_end).addr() - addr_of!(__start).addr()) as u64),
    ];

    let free_mem_ranges = mem_entries
        .iter()
        .filter(|data| data.is_usable())
        .map(|data| data.base_addr..data.base_addr + data.len)
        .flat_map(|range| SubtractRangesIterator::new(range, used_ranges.iter().cloned()))
        .collect::<heapless::Vec<_, _>>();
    let free_mem_ranges = INITIAL_FREE_MEM.call_once(|| free_mem_ranges);

    log::debug!("free mem ranges: {free_mem_ranges:#X?}.");

    let mut pmm = InitialPmm::new(&free_mem_ranges);

    // Offset map everything
    let top_level_page_table_phys_addr = Cr3::read().0.start_address().as_u64();
    // Safety: offset and page table is valid
    let mut pt = unsafe {
        TopLevelPageTable::new(
            OFFSET_MAP_VIRT_ADDR,
            top_level_page_table_phys_addr,
            common::paging::TopLevel::Maps256T,
        )
    };
    let mapping_size = LeafMappingSize::max_supported();
    let map_phys_end = free_mem_ranges.last().unwrap().end;
    let n_pages = map_phys_end.div_ceil(mapping_size.byte_size());
    for i in 0..n_pages {
        let phys_addr = mapping_size.byte_size() * i;
        let mapping = LeafMapping::new(
            mapping_size,
            OFFSET_MAP_VIRT_ADDR + phys_addr,
            phys_addr,
            LeafMappingFlags {
                writable: true,
                executable: true,
                user_mode_accessible: false,
                pat_index: WRITE_BACK_INDEX,
            },
        );
        unsafe { pt.ensure_mapped_leaf(mapping, &mut InitialScratchTablesIterator::new(&mut pmm)) };
    }

    MEMORY.call_once(|| {
        Mutex::new(Memory {
            pmm,
            vmm: Vmm::default(),
            pt,
        })
    });
}

pub fn alloc_phys(size: u64, align: u64) -> Option<u64> {
    MEMORY.get().unwrap().lock().pmm.allocate(size, align)
}

pub fn map_phys(addr: u64, len: u64, flags: LeafMappingFlags) -> Result<u64, MapPhysError> {
    let mut memory = MEMORY.get().unwrap().lock();
    let memory = &mut *memory;
    let mapping_size = LeafMappingSize::max_supported();
    let phys_start_addr = addr / mapping_size.byte_size() * mapping_size.byte_size();
    let phys_end_addr = (addr + len).next_multiple_of(mapping_size.byte_size());
    let n_mappings = (phys_end_addr - phys_start_addr) / mapping_size.byte_size();
    let virt_start_addr = memory
        .vmm
        .alloc(
            mapping_size.byte_size() * n_mappings,
            mapping_size.byte_size(),
        )
        .ok_or(MapPhysError::OutOfVirtMem)?;
    for i in 0..n_mappings {
        let mapping = LeafMapping::new(
            mapping_size,
            virt_start_addr + mapping_size.byte_size() * i,
            phys_start_addr + mapping_size.byte_size() * i,
            flags,
        );
        log::trace!("mapping {mapping:X?}");
        if let Err(e) = unsafe {
            memory
                .pt
                .map_leaf(mapping, &mut ScratchTablesIterator::new(&mut memory.pmm))
        } {
            match e {
                MapError::AlreadyMapped => {
                    panic!("page should not have been already mapped. mapping: {mapping:#X?}")
                }
                MapError::OutOfScratchTables => return Err(MapPhysError::OutOfVirtMem),
            }
        }
        log::trace!("mapped");
    }
    Ok(virt_start_addr + (addr - phys_start_addr))
}

#[derive(Debug)]
pub enum MapPhysError {
    OutOfPhysMem,
    OutOfVirtMem,
}
