use core::{cmp::min, ops::Range, ptr::addr_of};

use bitmap_allocator::{BitAlloc, BitAlloc1M};
use common::{
    big_stage_api::BigStageEntryInfo,
    bios::{Int15Data, MemoryIterator},
};
use nodit::{Interval, NoditMap, NoditSet};
use spin::{Mutex, Once};
use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};

use crate::{
    __bss_end, __start,
    physical_memory::{MemoryType, PhysicalMemory},
    range_utils::{SubtractRangesIterator, subtract_range},
    virtual_memory::VirtualMemory,
};

pub struct Memory {
    pub phys: PhysicalMemory,
    pub virt: VirtualMemory,
}

pub static MEMORY: Once<Mutex<Memory>> = Once::new();

pub unsafe fn init(info: &BigStageEntryInfo) {
    let used_ranges = [
        (0..info.low_used_mem_len),
        (info.big_stage_phys_start
            ..info.big_stage_phys_start
                + (addr_of!(__bss_end).addr() - addr_of!(__start).addr()) as u64),
    ];

    let mut physical_memory = PhysicalMemory {
        map: NoditMap::new(),
    };

    for range in MemoryIterator::default()
        .map(|result| result.unwrap())
        .filter(Int15Data::is_usable)
        .map(|data| data.base_addr..data.base_addr + data.len)
        .flat_map(|range| SubtractRangesIterator::new(range, used_ranges.iter().cloned()))
    {
        log::info!("available mem range: {range:#X?}");
        physical_memory
            .map
            .insert_merge_touching_if_values_equal(range.into(), MemoryType::Free)
            .unwrap();
    }

    let virtual_memory = VirtualMemory {
        set: {
            let mut set = NoditSet::new();
            set.insert_merge_touching((0..info.low_used_mem_len).into())
                .unwrap();
            set.insert_merge_touching(
                (addr_of!(__start) as u64..(addr_of!(__bss_end) as u64).next_multiple_of(0x200000))
                    .into(),
            )
            .unwrap();
            set
        },
    };

    MEMORY.call_once(|| {
        Mutex::new(Memory {
            phys: physical_memory,
            virt: virtual_memory,
        })
    });
}
