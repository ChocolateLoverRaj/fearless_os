use raw_cpuid::CpuId;

use crate::paging::{page_table_entry::PageTableEntry, virt_addr::VirtAddr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafMappingSize {
    _4K,
    _2M,
    /// Not all 64-bit CPUs support this. Remember to check with cpuid.
    _1G,
}

impl LeafMappingSize {
    pub const fn byte_size(&self) -> u64 {
        match self {
            Self::_4K => 0x1000,
            Self::_2M => 0x1000 * 512,
            Self::_1G => 0x1000 * 512 * 512,
        }
    }
}

impl LeafMappingSize {
    pub fn max_supported() -> Self {
        if CpuId::new()
            .get_extended_processor_and_feature_identifiers()
            .is_some_and(|info| info.has_1gib_pages())
        {
            Self::_1G
        } else {
            Self::_2M
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LeafMapping {
    pub(crate) size: LeafMappingSize,
    pub(crate) virt_addr: u64,
    pub(crate) phys_addr: u64,
}

impl LeafMapping {
    pub fn new(size: LeafMappingSize, virt_addr: u64, phys_addr: u64) -> Self {
        assert!(virt_addr.is_multiple_of(size.byte_size()));
        assert!(phys_addr.is_multiple_of(size.byte_size()));
        Self {
            size,
            virt_addr,
            phys_addr,
        }
    }

    pub(crate) fn make_entry(&self, entry: &mut PageTableEntry) {
        let phys_addr_bits = CpuId::new()
            .get_processor_capacity_feature_info()
            .unwrap()
            .physical_address_bits();
        let (shift, page_size) = match self.size {
            LeafMappingSize::_4K => (12, false),
            LeafMappingSize::_2M => (12 + 9, true),
            LeafMappingSize::_1G => (12 + 9 + 9, true),
        };
        let phys_addr_mask = ((1u64 << phys_addr_bits) - 1) & !((1u64 << shift) - 1);
        *entry = PageTableEntry::new_with_raw_value(
            (entry.raw_value() & !phys_addr_mask) | (self.phys_addr & phys_addr_mask),
        );
        entry.set_page_size(page_size);
    }
}
