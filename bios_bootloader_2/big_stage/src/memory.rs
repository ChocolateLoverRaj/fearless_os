use core::{
    alloc::GlobalAlloc,
    fmt::Debug,
    iter,
    mem::MaybeUninit,
    ptr::{NonNull, addr_of, null_mut},
};

use alloc::boxed::Box;
use common::{
    BIG_STAGE_MAP_OFFSET,
    big_stage_api::BigStageEntryInfo,
    bios::{Int15Data, MemoryIterator},
    paging::{LeafMapping, LeafMappingSize, PageTable, ScratchPageTable, TopLevelPageTable},
};
use spin::{Mutex, Once};
use x86_64::registers::control::Cr3;

use crate::{
    __bss_end, __start,
    free_iterator::FreeIterator,
    linked_list::{LinkedList, LinkedListNode},
    range_utils::SubtractRangesIterator,
};

#[derive(Debug, Clone, Copy)]
struct UsablePhysMemNodeData {
    pub phys_start: u64,
    pub len: u64,
}

#[derive(Debug, Clone, Copy)]
struct UsedPhysMemNodeData {
    pub phys_start: u64,
    pub len: u64,
}

const FREE_USED_PHYS_MEM_NODES_CAPACITY: usize = 2;
const FREE_PAGE_TABLES_CAPACITY: usize = 2;

struct MemoryData {
    used_phys_mem_list: LinkedList<UsedPhysMemNodeData>,
    free_used_phys_mem_nodes: heapless::Vec<u64, FREE_USED_PHYS_MEM_NODES_CAPACITY>,
    usable_phys_mem_list: LinkedList<UsablePhysMemNodeData>,
    free_page_tables: heapless::Vec<u64, FREE_PAGE_TABLES_CAPACITY>,
    top_page_table: TopLevelPageTable,
}

impl MemoryData {
    fn ensure_mapped_internal(&mut self, phys_addr: u64, len: u64) {}

    fn ensure_mapped(&mut self, phys_addr: u64, len: u64) {
        let mapping_size = LeafMappingSize::max_supported();
        let start_phys_addr = phys_addr / mapping_size.byte_size() * mapping_size.byte_size();
        let end_phys_addr_exclusive = (phys_addr + len).next_multiple_of(mapping_size.byte_size());
        let n_mappings = (end_phys_addr_exclusive - start_phys_addr) / mapping_size.byte_size();
        for i in 0..n_mappings {
            let phys_addr = start_phys_addr + i * mapping_size.byte_size();
            let mapping =
                LeafMapping::new(mapping_size, BIG_STAGE_MAP_OFFSET + phys_addr, phys_addr);
            let mut scratch_tables = iter::from_fn(|| {
                Some(unsafe { ScratchPageTable::new(self.free_page_tables.pop()?) })
            });
            // TODO: Make sure there are enough page tables
            unsafe {
                self.top_page_table
                    .ensure_mapped_leaf(mapping, &mut scratch_tables)
            }
            .unwrap();
        }
    }

    fn find_free_phys_mem(&self, size: u64, align: u64) -> Option<u64> {
        FreeIterator::new(
            self.usable_phys_mem_list
                .into_iter()
                .map(|node| node.data.phys_start..node.data.phys_start + node.data.len),
            self.used_phys_mem_list
                .into_iter()
                .map(|node| node.data.phys_start..node.data.phys_start + node.data.len),
        )
        .find_map(|range| {
            let aligned_start = range.start.next_multiple_of(align);
            let would_be_end = aligned_start + size;
            if would_be_end <= range.end {
                Some(aligned_start)
            } else {
                None
            }
        })
    }

    /// Must have at least 1 free used phys mem node and 4 free page tables when calling this fn.
    fn refill_free_used_phys_mem_nodes(&mut self) {
        let node_phys_addr = self.free_used_phys_mem_nodes.pop().unwrap();
        let nodes_to_allocate =
            FREE_USED_PHYS_MEM_NODES_CAPACITY - self.free_used_phys_mem_nodes.len();
        let alloc_size = (nodes_to_allocate * size_of::<LinkedListNode<UsedPhysMemNodeData>>())
            .try_into()
            .unwrap();
        let nodes_phys_addr = self
            .find_free_phys_mem(
                alloc_size,
                align_of::<LinkedListNode<UsedPhysMemNodeData>>()
                    .try_into()
                    .unwrap(),
            )
            .expect("out of memory");
        let mut node_ptr = NonNull::new(
            (BIG_STAGE_MAP_OFFSET + node_phys_addr)
                as *mut MaybeUninit<LinkedListNode<UsedPhysMemNodeData>>,
        )
        .unwrap();
        let node_ptr = unsafe { node_ptr.as_mut() };
        let node_ptr = &raw mut *node_ptr.write(LinkedListNode::new(UsedPhysMemNodeData {
            phys_start: nodes_phys_addr,
            len: alloc_size,
        }));
        self.used_phys_mem_list
            .push_back_boxed(unsafe { Box::from_raw(node_ptr) });
        for i in 0..nodes_to_allocate {
            self.free_used_phys_mem_nodes
                .push(
                    nodes_phys_addr
                        + u64::try_from(i * size_of::<LinkedListNode<UsedPhysMemNodeData>>())
                            .unwrap(),
                )
                .unwrap();
        }
    }

    /// Must have at least 1 free used phys mem node and 2 page tables when calling this.
    fn refill_free_page_tabes(&mut self) {
        todo!()
    }
}

pub struct Memory {
    data: Once<Mutex<MemoryData>>,
}

impl Memory {
    pub fn ensure_mapped_phys(&self, phys_addr: u64, len: u64) {
        self.data
            .get()
            .unwrap()
            .lock()
            .ensure_mapped(phys_addr, len);
    }
}

unsafe impl GlobalAlloc for Memory {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut data = self.data.get().unwrap().lock();

        let layout_size_u64 = u64::try_from(layout.size()).unwrap();
        let Some(suitable_phys_start) =
            data.find_free_phys_mem(layout_size_u64, u64::try_from(layout.align()).unwrap())
        else {
            return null_mut();
        };

        let node = {
            let mut node_ptr: NonNull<MaybeUninit<LinkedListNode<UsedPhysMemNodeData>>> =
                NonNull::new(
                    (BIG_STAGE_MAP_OFFSET + data.free_used_phys_mem_nodes.pop().unwrap())
                        as *mut MaybeUninit<LinkedListNode<UsedPhysMemNodeData>>,
                )
                .unwrap();
            let node_ptr = unsafe { node_ptr.as_mut() };
            let node_ptr = &raw mut *node_ptr.write(LinkedListNode::new(UsedPhysMemNodeData {
                phys_start: suitable_phys_start,
                len: layout_size_u64,
            }));
            unsafe { Box::from_raw(node_ptr) }
        };
        data.used_phys_mem_list.push_back_boxed(node);

        log::info!("suitable phys start: {suitable_phys_start:#X}");

        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
    }
}

#[global_allocator]
pub static MEMORY: Memory = Memory { data: Once::new() };

/// # Safety
/// - Memory at addr must be valid and owned for [LinkedListNode<T>] based on the iterator len.
/// - See safety of [`Box::from_raw`].
unsafe fn write_contiguous_linked_list<T, I: IntoIterator<Item = T>>(
    addr: NonNull<MaybeUninit<LinkedListNode<T>>>,
    iterator: I,
) -> LinkedList<T> {
    let mut linked_list = LinkedList::new();
    for (i, item) in iterator.into_iter().enumerate() {
        let mut ptr = unsafe { addr.add(i) };
        let ptr = unsafe { ptr.as_mut() };
        let ptr = &raw mut *ptr.write(LinkedListNode {
            data: item,
            next: None,
        });
        linked_list.push_back_boxed(unsafe { Box::from_raw(ptr) });
    }
    linked_list
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
            BIG_STAGE_MAP_OFFSET,
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
                    + u64::try_from(addr_of!(*page_table).addr() - addr_of!(__start).addr())
                        .unwrap(),
            )
        }
    });

    // Determine the number of usable mem nodes, which we will call N
    let usable_mem_nodes_count = free_mem().count();

    let used_phys_mem_nodes_to_allocate = 3 + FREE_PAGE_TABLES_CAPACITY;

    let page_tables_to_allocate = FREE_PAGE_TABLES_CAPACITY;

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
                    * u64::try_from(used_phys_mem_nodes_to_allocate).unwrap();

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
            let potential_page_tables_end = potential_page_tables_start
                + 0x1000 * u64::try_from(page_tables_to_allocate).unwrap();

            potential_page_tables_end <= range.end
        })
        .unwrap();

    log::info!("Found first chunk to use: {first_chunk_to_use:#X?}");

    // Make sure that the chunk is mapped (we will assume we don't need to map >1G)
    let mut map_data = |phys_start: u64, len: u64| {
        let page_start_phys = phys_start / mapping_size.byte_size() * mapping_size.byte_size();
        let page_start_virt = BIG_STAGE_MAP_OFFSET + page_start_phys;
        let page_end_phys = (phys_start + len).next_multiple_of(mapping_size.byte_size());
        let map_count = (page_end_phys - page_start_phys) / mapping_size.byte_size();
        for i in 0..map_count {
            unsafe {
                let mapping = LeafMapping::new(
                    mapping_size,
                    page_start_virt + i * mapping_size.byte_size(),
                    page_start_phys + i * mapping_size.byte_size(),
                );
                log::info!("mapping {mapping:#X?}");
                pt.ensure_mapped_leaf(mapping, &mut scratch_tables)
            }
            .unwrap();
        }
    };

    let used_phys_mem_nodes_phys_start_addr = first_chunk_to_use.start.next_multiple_of(
        align_of::<LinkedListNode<UsedPhysMemNodeData>>()
            .try_into()
            .unwrap(),
    );
    let used_phys_mem_nodes_len = u64::try_from(used_phys_mem_nodes_to_allocate).unwrap()
        * u64::try_from(size_of::<LinkedListNode<UsedPhysMemNodeData>>()).unwrap();
    let used_phys_mem_nodes_phys_end_addr =
        used_phys_mem_nodes_phys_start_addr + u64::try_from(used_phys_mem_nodes_len).unwrap();

    let usable_phys_mem_nodes_phys_start_addr = used_phys_mem_nodes_phys_end_addr.next_multiple_of(
        align_of::<LinkedListNode<UsablePhysMemNodeData>>()
            .try_into()
            .unwrap(),
    );
    let usable_phys_mem_nodes_len = u64::try_from(usable_mem_nodes_count).unwrap()
        * u64::try_from(size_of::<LinkedListNode<UsablePhysMemNodeData>>()).unwrap();
    let usable_phys_mem_nodes_phys_end_addr =
        usable_phys_mem_nodes_phys_start_addr + usable_phys_mem_nodes_len;

    let page_tables_phys_start_addr = usable_phys_mem_nodes_phys_end_addr.next_multiple_of(0x1000);

    // Create used phys mem node for used phys mem nodes
    map_data(used_phys_mem_nodes_phys_start_addr, used_phys_mem_nodes_len);
    let used_phys_mem_nodes_ptr =
        NonNull::new((used_phys_mem_nodes_phys_start_addr + BIG_STAGE_MAP_OFFSET) as *mut _)
            .unwrap();
    // Safety: the memory is mapped and we have exclusive ownership of it
    let used_phys_mem_nodes = [
        UsedPhysMemNodeData {
            phys_start: used_phys_mem_nodes_phys_start_addr,
            len: u64::try_from(used_phys_mem_nodes_to_allocate).unwrap()
                * u64::try_from(size_of::<LinkedListNode<UsedPhysMemNodeData>>()).unwrap(),
        },
        UsedPhysMemNodeData {
            phys_start: usable_phys_mem_nodes_phys_start_addr,
            len: u64::try_from(usable_mem_nodes_count).unwrap()
                * u64::try_from(size_of::<LinkedListNode<UsablePhysMemNodeData>>()).unwrap(),
        },
        UsedPhysMemNodeData {
            phys_start: page_tables_phys_start_addr,
            len: u64::try_from(page_tables_to_allocate).unwrap() * 0x1000,
        },
    ];
    let used_phys_mem_list =
        unsafe { write_contiguous_linked_list(used_phys_mem_nodes_ptr, used_phys_mem_nodes) };
    log::info!("used_phys_mem_list: {used_phys_mem_list:#X?}");

    let free_used_phys_mem_nodes = heapless::Vec::<_, 2>::from([
        used_phys_mem_nodes_phys_start_addr
            + u64::try_from(size_of::<LinkedListNode<UsedPhysMemNodeData>>()).unwrap() * 3,
        used_phys_mem_nodes_phys_start_addr
            + u64::try_from(size_of::<LinkedListNode<UsedPhysMemNodeData>>()).unwrap() * 4,
    ]);
    log::info!("free_used_phys_mem_nodes={free_used_phys_mem_nodes:#X?}");

    // Create useable phys mem nodes
    map_data(
        usable_phys_mem_nodes_phys_start_addr,
        usable_phys_mem_nodes_len,
    );
    let usable_phys_mem_nodes_ptr =
        NonNull::new((BIG_STAGE_MAP_OFFSET + usable_phys_mem_nodes_phys_start_addr) as *mut _)
            .unwrap();
    // Safety: The memory is already mapped and we have ownership of it
    let usable_phys_mem_list = unsafe {
        write_contiguous_linked_list(
            usable_phys_mem_nodes_ptr,
            free_mem().map(|range| UsablePhysMemNodeData {
                phys_start: range.start,
                len: range.end - range.start,
            }),
        )
    };
    log::info!("usable_phys_mem_list: {usable_phys_mem_list:#X?}");

    let free_page_tables = heapless::Vec::<_, 2>::from_iter(
        (0..page_tables_to_allocate)
            .map(|i| page_tables_phys_start_addr + 0x1000 * u64::try_from(i).unwrap()),
    );
    log::info!("free page tables: {free_page_tables:#X?}.");

    let memory = MemoryData {
        used_phys_mem_list,
        free_used_phys_mem_nodes,
        usable_phys_mem_list,
        free_page_tables,
        top_page_table: pt,
    };

    MEMORY.data.call_once(|| Mutex::new(memory));

    // let b = Box::new(3);
    // log::info!("Box: {b:p} {b}");
}
