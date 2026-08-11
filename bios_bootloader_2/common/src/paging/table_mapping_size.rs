#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableMappingSize {
    _2M,
    _1G,
    _512G,
    _256T,
}

impl TableMappingSize {
    pub fn size_bytes(&self) -> u64 {
        match self {
            TableMappingSize::_2M => 0x1000 * 512,
            TableMappingSize::_1G => 0x1000 * 512 * 512,
            TableMappingSize::_512G => 0x1000 * 512 * 512 * 512,
            TableMappingSize::_256T => 0x1000 * 512 * 512 * 512 * 512,
        }
    }
}
