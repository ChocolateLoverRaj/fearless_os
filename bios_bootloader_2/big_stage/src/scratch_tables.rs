use core::{mem::MaybeUninit, ptr::NonNull};

use common::{
    OFFSET_MAP_VIRT_ADDR,
    paging::{PageTable, ScratchPageTable},
};
use zerocopy::FromZeros;

use crate::initial_pmm::InitialPmm;

pub struct InitialScratchTablesIterator<'a, 'b> {
    pmm: &'a mut InitialPmm<'b>,
}

impl<'a, 'b> InitialScratchTablesIterator<'a, 'b> {
    pub fn new(pmm: &'a mut InitialPmm<'b>) -> Self {
        Self { pmm }
    }
}

impl Iterator for InitialScratchTablesIterator<'_, '_> {
    type Item = ScratchPageTable;

    fn next(&mut self) -> Option<Self::Item> {
        let addr = self.pmm.allocate(0x1000, 0x1000)?;
        if addr >= 0x40000000 {
            panic!(
                "could not allocate page table because allocated page was outside of low 1 GiB range."
            );
        }
        let mut table_ptr =
            NonNull::new((OFFSET_MAP_VIRT_ADDR + addr) as *mut MaybeUninit<PageTable>).unwrap();
        let table_ptr = unsafe { table_ptr.as_mut() };
        table_ptr.zero();
        Some(unsafe { ScratchPageTable::new(addr) })
    }
}

pub struct ScratchTablesIterator<'a, 'b> {
    pmm: &'a mut InitialPmm<'b>,
}

impl<'a, 'b> ScratchTablesIterator<'a, 'b> {
    pub fn new(pmm: &'a mut InitialPmm<'b>) -> Self {
        Self { pmm }
    }
}

impl Iterator for ScratchTablesIterator<'_, '_> {
    type Item = ScratchPageTable;

    fn next(&mut self) -> Option<Self::Item> {
        let addr = self.pmm.allocate(0x1000, 0x1000)?;
        let mut table_ptr =
            NonNull::new((OFFSET_MAP_VIRT_ADDR + addr) as *mut MaybeUninit<PageTable>).unwrap();
        let table_ptr = unsafe { table_ptr.as_mut() };
        table_ptr.zero();
        Some(unsafe { ScratchPageTable::new(addr) })
    }
}
