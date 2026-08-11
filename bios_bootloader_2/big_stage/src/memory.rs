use core::{
    cmp::min,
    mem::MaybeUninit,
    ops::Range,
    ptr::{NonNull, addr_of},
};

use bitmap_allocator::{BitAlloc, BitAlloc1M};
use common::{
    big_stage_api::BigStageEntryInfo,
    bios::{Int15Data, MemoryIterator},
    paging::{
        LeafMapping, LeafMappingSize, MapError, PageTable, ScratchPageTable, TopLevelPageTable,
    },
};
use nodit::{Interval, NoditMap, NoditSet};
use spin::{Mutex, Once};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB, page},
};

use crate::{
    __bss_end, __start, MAP_OFFSET,
    physical_memory::{MemoryType, PhysicalMemory},
    range_utils::{SubtractRangesIterator, subtract_range},
    virtual_memory::VirtualMemory,
};

struct UsablePhysMemNodeData {
    pub phys_start: u64,
    pub len: u64,
}

struct UsedPhysMemNodeData {
    pub phys_start: u64,
    pub len: u64,
}

struct LinkedListNode<T> {
    pub data: T,
    pub next_phys_addr: Option<u64>,
}

pub unsafe fn init(info: &BigStageEntryInfo) {
    let used_ranges = [
        (0..info.low_used_mem_len),
        (info.big_stage_phys_start
            ..info.big_stage_phys_start
                + (addr_of!(__bss_end).addr() - addr_of!(__start).addr()) as u64),
    ];

    let free_mem = || {
        MemoryIterator::default()
            .map(|result| result.unwrap())
            .filter(Int15Data::is_usable)
            .map(|data| data.base_addr..data.base_addr + data.len)
            .flat_map(|range| SubtractRangesIterator::new(range, used_ranges.iter().cloned()))
    };

    let top_level_page_table_phys_addr = Cr3::read().0.start_address().as_u64();
    // Safety: offset and page table is valid
    let mut pt = unsafe {
        TopLevelPageTable::new(
            0,
            top_level_page_table_phys_addr,
            common::paging::TopLevel::Maps256T,
        )
    };

    let mapping_size = LeafMappingSize::max_supported();

    static PRE_ALLOCATED_PAGE_TABLES: [PageTable; 4] = [PageTable::new(); _];
    let mut scratch_tables = PRE_ALLOCATED_PAGE_TABLES.iter().map(|page_table| {
        // Safety: we own the page table and it's aligned
        unsafe {
            // FIXME: Only works if this is mapped, which is only the case if it happens to be in <1 GiB phys mem
            ScratchPageTable::new(
                info.big_stage_phys_start
                    + u64::try_from((addr_of!(*page_table).addr() - addr_of!(__start).addr()))
                        .unwrap(),
            )
        }
    });

    // Determine the number of usable mem nodes, which we will call N
    let usable_mem_nodes_count = free_mem().count();

    let used_phys_mem_nodes_to_allocate = 3 + 2;

    let page_tables_to_allocate = 2;

    // Find a chunk of free phys mem that is large enough to contain (considering alignment) N + 2 used mem nodes, 2 page tables.
    let first_chunk_to_use = free_mem()
        .find(|range| {
            let potential_used_mem_nodes_start = range.start.next_multiple_of(
                align_of::<LinkedListNode<UsedPhysMemNodeData>>()
                    .try_into()
                    .unwrap(),
            );
            let potential_used_mem_nodes_end = potential_used_mem_nodes_start
                + u64::try_from(size_of::<LinkedListNode<UsedPhysMemNodeData>>()).unwrap()
                    * used_phys_mem_nodes_to_allocate;

            let potential_usable_mem_nodes_start = potential_used_mem_nodes_end.next_multiple_of(
                align_of::<LinkedListNode<UsablePhysMemNodeData>>()
                    .try_into()
                    .unwrap(),
            );
            let potential_usable_mem_nodes_end = potential_usable_mem_nodes_start
                + u64::try_from(size_of::<LinkedListNode<UsablePhysMemNodeData>>()).unwrap()
                    * u64::try_from(usable_mem_nodes_count).unwrap();

            let potential_page_tables_start =
                potential_usable_mem_nodes_end.next_multiple_of(0x1000);
            let potential_page_tables_end =
                potential_page_tables_start + 0x1000 * page_tables_to_allocate;

            potential_page_tables_end <= range.end
        })
        .unwrap();

    log::info!("Found first chunk to use: {first_chunk_to_use:#X?}");

    // Make sure that the chunk is mapped (we will assume we don't need to map >1G)
    let mut map_data = |phys_start: u64, len: u64| {
        let page_start_phys = phys_start / mapping_size.byte_size() * mapping_size.byte_size();
        let page_start_virt = MAP_OFFSET + page_start_phys;
        let page_end_phys = (phys_start + len).next_multiple_of(mapping_size.byte_size());
        let map_count = (page_end_phys - page_start_phys) / mapping_size.byte_size();
        for i in 0..map_count {
            let result = unsafe {
                let mapping = LeafMapping::new(
                    mapping_size,
                    page_start_virt + i * mapping_size.byte_size(),
                    page_start_phys + i * mapping_size.byte_size(),
                );
                log::info!("mapping {mapping:#X?}");
                pt.map_leaf(mapping, &mut scratch_tables)
            };
            match result {
                Ok(_)
                | Err(MapError::AlreadyMapped {
                    table: _,
                    entry_index: _,
                }) => {}
                Err(e) => panic!("{e:?}"),
            }
        }
    };

    let used_phys_mem_nodes_phys_start_addr = first_chunk_to_use.start.next_multiple_of(
        align_of::<LinkedListNode<UsedPhysMemNodeData>>()
            .try_into()
            .unwrap(),
    );
    let used_phys_mem_nodes_len = used_phys_mem_nodes_to_allocate
        * u64::try_from(size_of::<LinkedListNode<UsedPhysMemNodeData>>()).unwrap();
    let used_phys_mem_nodes_phys_end_addr =
        used_phys_mem_nodes_phys_start_addr + used_phys_mem_nodes_len;

    let usable_phys_mem_nodes_phys_start_addr = used_phys_mem_nodes_phys_end_addr.next_multiple_of(
        align_of::<LinkedListNode<UsablePhysMemNodeData>>()
            .try_into()
            .unwrap(),
    );
    let usable_phys_mem_nodes_phys_end_addr = usable_phys_mem_nodes_phys_start_addr
        + u64::try_from(usable_mem_nodes_count).unwrap()
            * u64::try_from(size_of::<LinkedListNode<UsablePhysMemNodeData>>()).unwrap();

    let page_tables_phys_start_addr = usable_phys_mem_nodes_phys_end_addr.next_multiple_of(0x1000);
    let page_tables_phys_end_addr = page_tables_phys_start_addr + 0x1000 * page_tables_to_allocate;

    // Create used phys mem node for used phys mem nodes
    map_data(used_phys_mem_nodes_phys_start_addr, used_phys_mem_nodes_len);
    let mut used_phys_mem_nodes_ptr = NonNull::slice_from_raw_parts(
        NonNull::new(
            (used_phys_mem_nodes_phys_start_addr + MAP_OFFSET)
                as *mut MaybeUninit<LinkedListNode<UsedPhysMemNodeData>>,
        )
        .unwrap(),
        used_phys_mem_nodes_to_allocate.try_into().unwrap(),
    );
    // Safety: the memory is mapped and we have exclusive ownership of it
    let used_phys_mem_nodes = unsafe { used_phys_mem_nodes_ptr.as_mut() };
    used_phys_mem_nodes[0].write(LinkedListNode {
        data: UsedPhysMemNodeData {
            phys_start: used_phys_mem_nodes_phys_start_addr,
            len: used_phys_mem_nodes_to_allocate
                * u64::try_from(size_of::<LinkedListNode<UsedPhysMemNodeData>>()).unwrap(),
        },
        next_phys_addr: Some(
            used_phys_mem_nodes_phys_start_addr
                + u64::try_from(size_of::<LinkedListNode<UsedPhysMemNodeData>>()).unwrap(),
        ),
    });
    used_phys_mem_nodes[1].write(LinkedListNode {
        data: UsedPhysMemNodeData {
            phys_start: usable_phys_mem_nodes_phys_start_addr,
            len: u64::try_from(usable_mem_nodes_count).unwrap()
                * u64::try_from(size_of::<LinkedListNode<UsablePhysMemNodeData>>()).unwrap(),
        },
        next_phys_addr: Some(
            used_phys_mem_nodes_phys_start_addr
                + u64::try_from(size_of::<LinkedListNode<UsedPhysMemNodeData>>()).unwrap() * 2,
        ),
    });
    used_phys_mem_nodes[2].write(LinkedListNode {
        data: UsedPhysMemNodeData {
            phys_start: page_tables_phys_start_addr,
            len: 0x1000 * page_tables_to_allocate,
        },
        next_phys_addr: None,
    });

    todo!()
}
