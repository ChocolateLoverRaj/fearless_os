use core::{ops::Range, ptr::addr_of};

use bios_bootloader_common::{
    OFFSET_MAP_LEN, OFFSET_MAP_VIRT_ADDR, big_stage_api::BigStageEntryInfo, bios::BiosFns,
};
use heapless::Vec;
use kernel_common::{
    memory,
    paging::{TopLevel, TopLevelPageTable},
    vmm::VirtMemRange,
};
use spin::Once;
use x86_64::registers::control::Cr3;

use crate::{
    __bss_end, __start, DYNAMIC_VIRT_ADDR, DYNAMIC_VIRT_LEN,
    range_utils::{SubtractRangesIterator, is_overlap},
};

static INITIAL_FREE_MEM: Once<Vec<Range<u64>, 32>> = Once::new();

/// # Safety
///
/// Must be called exactly once.
pub unsafe fn init(info: &BigStageEntryInfo, bios_fns: BiosFns) {
    let mut mem_entries = bios_fns
        .memory()
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

    let top_level_page_table_phys_addr = Cr3::read().0.start_address().as_u64();
    // Safety: offset and page table is valid
    let pt = unsafe {
        TopLevelPageTable::new(
            OFFSET_MAP_VIRT_ADDR,
            top_level_page_table_phys_addr,
            TopLevel::Maps256T,
        )
    };

    unsafe {
        memory::init(
            free_mem_ranges,
            VirtMemRange {
                addr: OFFSET_MAP_VIRT_ADDR,
                len: 0x40000000,
            },
            VirtMemRange {
                addr: DYNAMIC_VIRT_ADDR,
                len: DYNAMIC_VIRT_LEN,
            },
            VirtMemRange {
                addr: OFFSET_MAP_VIRT_ADDR,
                len: OFFSET_MAP_LEN,
            },
            Some(pt),
        )
    };
}
