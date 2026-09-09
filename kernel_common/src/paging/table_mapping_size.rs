use crate::paging::entry_size::EntrySize;

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

impl TryFrom<EntrySize> for TableMappingSize {
    type Error = Is4KEntry;

    fn try_from(value: EntrySize) -> Result<Self, Self::Error> {
        match value {
            EntrySize::_256T => Ok(Self::_256T),
            EntrySize::_512G => Ok(Self::_512G),
            EntrySize::_1G => Ok(Self::_1G),
            EntrySize::_2M => Ok(Self::_2M),
            EntrySize::_4K => Err(Is4KEntry),
        }
    }
}

/// A 4K entry can only be a leaf mapping. It can't be a table mapping.
#[derive(Debug, Clone, Copy)]
pub struct Is4KEntry;
