use raw_cpuid::CpuId;

use crate::paging::entry_size::EntrySize;

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

impl TryFrom<EntrySize> for LeafMappingSize {
    type Error = LeafMappingSizeNotSupported;

    fn try_from(value: EntrySize) -> Result<Self, Self::Error> {
        match value {
            EntrySize::_256T => Err(LeafMappingSizeNotSupported),
            EntrySize::_512G => Err(LeafMappingSizeNotSupported),
            EntrySize::_1G => Ok(Self::_1G),
            EntrySize::_2M => Ok(Self::_2M),
            EntrySize::_4K => Ok(Self::_4K),
        }
    }
}

/// 4 KiB and 2 MiB mappings - always supported.
/// 1 GiB mappings - supported on most but not all processors.
/// >1 GiB mappings - does not exist in the specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LeafMappingSizeNotSupported;
