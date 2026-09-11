use spin::{Mutex, Once};
use x86_64::{
    PhysAddr,
    registers::control::{Cr3, Cr3Flags, Efer, EferFlags},
    structures::paging::PhysFrame,
};

use crate::{
    initial_pmm::{InitialFreeMem, InitialPmm},
    paging::{
        LeafMapping, LeafMappingFlags, LeafMappingSize, MapError, TopLevel, TopLevelPageTable,
    },
    pat::{self, WRITE_BACK_INDEX},
    scratch_tables::ScratchTablesAllocator,
    vmm::{VirtMemRange, Vmm},
};

struct Memory {
    pmm: InitialPmm<'static>,
    vmm: Vmm,
    pt: TopLevelPageTable,
    offset_map_range: VirtMemRange,
}

static MEMORY: Once<Mutex<Memory>> = Once::new();

/// # Safety
///
/// Must be called exactly once.
pub unsafe fn init(
    initial_free_mem: &'static mut dyn InitialFreeMem,
    initial_offset_map: VirtMemRange,
    dynamic_virt_info: VirtMemRange,
    offset_map_range: VirtMemRange,
    top_level_page_table: Option<TopLevelPageTable>,
) {
    // Safety: doesn't break any existing mappings
    unsafe { pat::init() };

    // Enable no-execute flag
    unsafe { Efer::update(|efer| efer.insert(EferFlags::NO_EXECUTE_ENABLE)) };

    let map_phys_end = initial_free_mem.phys_end_addr();
    let mut pmm = InitialPmm::new(initial_free_mem);

    // Offset map everything
    let mut scratch_tables_allocator = ScratchTablesAllocator::new(&mut pmm, initial_offset_map);
    // Safety: offset and page table is valid
    let top_level_page_table_is_none = top_level_page_table.is_none();
    let mut pt = top_level_page_table.unwrap_or_else(|| unsafe {
        TopLevelPageTable::new(
            initial_offset_map.addr,
            scratch_tables_allocator.next().unwrap().addr,
            TopLevel::Maps256T,
        )
    });
    let mapping_size = LeafMappingSize::max_supported();
    let n_pages = map_phys_end.div_ceil(mapping_size.byte_size());
    for i in 0..n_pages {
        let phys_addr = mapping_size.byte_size() * i;
        let mapping = LeafMapping::new(
            mapping_size,
            offset_map_range.addr + phys_addr,
            phys_addr,
            LeafMappingFlags {
                writable: true,
                executable: true,
                user_mode_accessible: false,
                pat_index: WRITE_BACK_INDEX,
            },
        );
        unsafe { pt.ensure_mapped_leaf(mapping, &mut scratch_tables_allocator) }.unwrap();
    }

    if top_level_page_table_is_none {
        unsafe {
            Cr3::write(
                PhysFrame::from_start_address(PhysAddr::new(pt.phys_addr())).unwrap(),
                Cr3Flags::empty(),
            )
        };
    }

    MEMORY.call_once(|| {
        Mutex::new(Memory {
            pmm,
            vmm: Vmm::new(dynamic_virt_info),
            pt,
            offset_map_range,
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
            memory.pt.map_leaf(
                mapping,
                &mut ScratchTablesAllocator::new(&mut memory.pmm, memory.offset_map_range),
            )
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
