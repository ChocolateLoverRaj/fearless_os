use core::{mem::MaybeUninit, ptr::NonNull};

use zerocopy::FromZeros;

use crate::{
    initial_pmm::InitialPmm,
    paging::{PageTable, ScratchPageTable},
    vmm::VirtMemRange,
};

pub struct ScratchTablesAllocator<'a, 'b> {
    pmm: &'a mut InitialPmm<'b>,
    offset_map_info: VirtMemRange,
}

impl<'a, 'b> ScratchTablesAllocator<'a, 'b> {
    pub fn new(pmm: &'a mut InitialPmm<'b>, offset_map_info: VirtMemRange) -> Self {
        Self {
            pmm,
            offset_map_info,
        }
    }
}

impl Iterator for ScratchTablesAllocator<'_, '_> {
    type Item = ScratchPageTable;

    fn next(&mut self) -> Option<Self::Item> {
        let addr = self.pmm.allocate(0x1000, 0x1000)?;
        if addr >= self.offset_map_info.len {
            panic!(
                "could not allocate page table because allocated page was outside initial offset mapped mem"
            );
        }
        let mut table_ptr =
            NonNull::new((self.offset_map_info.addr + addr) as *mut MaybeUninit<PageTable>)
                .unwrap();
        let table_ptr = unsafe { table_ptr.as_mut() };
        table_ptr.zero();
        Some(unsafe { ScratchPageTable::new(addr) })
    }
}
