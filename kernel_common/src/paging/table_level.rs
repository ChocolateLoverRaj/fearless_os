use crate::paging::{entry_size::EntrySize, top_level::TopLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableLevel {
    Maps128P,
    Maps256T,
    Maps512G,
    Maps1G,
    Maps2M,
}

impl From<TopLevel> for TableLevel {
    fn from(level: TopLevel) -> Self {
        match level {
            TopLevel::Maps128P => Self::Maps128P,
            TopLevel::Maps256T => Self::Maps256T,
        }
    }
}

impl TableLevel {
    pub fn entry_size(&self) -> EntrySize {
        match self {
            Self::Maps128P => EntrySize::_256T,
            Self::Maps256T => EntrySize::_512G,
            Self::Maps512G => EntrySize::_1G,
            Self::Maps1G => EntrySize::_2M,
            Self::Maps2M => EntrySize::_4K,
        }
    }

    pub fn child(&self) -> Option<Self> {
        match self {
            Self::Maps128P => Some(Self::Maps256T),
            Self::Maps256T => Some(Self::Maps512G),
            Self::Maps512G => Some(Self::Maps1G),
            Self::Maps1G => Some(Self::Maps2M),
            Self::Maps2M => None,
        }
    }
}
