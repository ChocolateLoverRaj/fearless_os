use common::paging::ScratchPageTable;

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
        Some(unsafe { ScratchPageTable::new(addr) })
    }
}
