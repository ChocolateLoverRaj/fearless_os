use crate::paging::{LeafMappingFlags, LeafMappingSize};

#[derive(Debug, Clone, Copy)]
pub struct LeafMapping {
    pub(crate) size: LeafMappingSize,
    pub(crate) virt_addr: u64,
    pub(crate) phys_addr: u64,
    pub(crate) flags: LeafMappingFlags,
}

impl LeafMapping {
    pub fn new(
        size: LeafMappingSize,
        virt_addr: u64,
        phys_addr: u64,
        flags: LeafMappingFlags,
    ) -> Self {
        assert!(virt_addr.is_multiple_of(size.byte_size()));
        assert!(phys_addr.is_multiple_of(size.byte_size()));
        Self {
            size,
            virt_addr,
            phys_addr,
            flags,
        }
    }
}
