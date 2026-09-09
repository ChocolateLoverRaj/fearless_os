use crate::paging::{table_mapping_size::TableMappingSize, virt_addr::VirtAddr};

pub struct TableMappingVirtAddr {
    pub(crate) addr: VirtAddr,
    pub(crate) size: TableMappingSize,
}

impl TableMappingVirtAddr {
    pub fn new(addr: u64, size: TableMappingSize) -> Self {
        assert!(addr.is_multiple_of(size.size_bytes()));
        Self {
            addr: VirtAddr::new_with_raw_value(addr),
            size,
        }
    }
}
