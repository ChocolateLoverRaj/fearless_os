pub struct ScratchPageTable {
    pub(crate) addr: u64,
}

impl ScratchPageTable {
    /// # Safety
    /// The address must point to an exclusive physical address of a page table and must be accessible from virt mem.
    /// The table must be zeroed.
    pub unsafe fn new(addr: u64) -> Self {
        assert!(addr.is_multiple_of(0x1000));
        Self { addr }
    }
}
