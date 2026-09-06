use arbitrary_int::{u22, u31, u40};
use bitbybit::bitfield;

use crate::paging::{LeafMappingFlags, LeafMappingSize, TableMappingSize, entry_size::EntrySize};

pub trait EntryCommon {
    fn phys_addr_u64(&self) -> u64;
    /// Entries have configuration, such as phys addr, permissions, and cache behavior, and they also have runtime status info such as accessed and dirty. This only compares the configuration bits.
    fn config_eq(&self, other: &Self) -> bool;
}

/// PTE - Table 5-20. Format of a Page-Table Entry that Maps a 4-KByte Page
#[bitfield(u64, debug)]
#[derive(Default)]
pub struct PageTableEntry4KToPage {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    writable: bool,
    #[bit(2, rw)]
    user_mode_accessible: bool,
    #[bit(3, rw)]
    page_level_write_through: bool,
    #[bit(4, rw)]
    page_level_cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(6, rw)]
    dirty: bool,
    #[bit(7, rw)]
    pat: bool,
    #[bit(8, rw)]
    global: bool,
    #[bits(12..=51, rw)]
    phys_addr: u40,
    #[bit(63, rw)]
    not_executable: bool,
}

impl PageTableEntry4KToPage {
    /// You must guarantee that this entry is actually a 4 KiB entry.
    pub fn from_common(common: PageTableEntryCommon) -> Self {
        Self::new_with_raw_value(common.raw_value)
    }
}

impl EntryCommon for PageTableEntry4KToPage {
    fn phys_addr_u64(&self) -> u64 {
        self.phys_addr().value() << 12
    }

    fn config_eq(&self, other: &Self) -> bool {
        self.present() == other.present()
            && self.writable() == other.writable()
            && self.user_mode_accessible() == other.user_mode_accessible()
            && self.page_level_write_through() == other.page_level_write_through()
            && self.page_level_cache_disable() == other.page_level_cache_disable()
            && self.pat() == other.pat()
            && self.global() == other.global()
            && self.phys_addr() == other.phys_addr()
            && self.not_executable() == other.not_executable()
    }
}

/// Common bits for entries that map 2M (PDE) and 1G (PDPTE).
#[bitfield(u64, debug)]
pub struct PageTableEntry2M1GCommon {
    /// Technically called "page size" by Intel.
    #[bit(7, rw)]
    is_leaf_mapping: bool,
}

impl PageTableEntry2M1GCommon {
    /// You must guarantee that this entry is actually a 1 GiB or 2 MiB entry.
    fn from_common(common: PageTableEntryCommon) -> Self {
        Self::new_with_raw_value(common.raw_value)
    }
}

/// PDE - Table 5-19. Format of a Page-Directory Entry that References a Page Table
#[bitfield(u64, debug)]
#[derive(Default)]
pub struct PageTableEntry2MToTable {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    writable: bool,
    #[bit(2, rw)]
    user_mode_accessible: bool,
    #[bit(3, rw)]
    page_level_write_through: bool,
    #[bit(4, rw)]
    page_level_cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(7, rw)]
    page_size: bool,
    #[bits(12..=51, rw)]
    phys_addr: u40,
    #[bit(63, rw)]
    not_executable: bool,
}

impl EntryCommon for PageTableEntry2MToTable {
    fn phys_addr_u64(&self) -> u64 {
        self.phys_addr().value() << 12
    }

    fn config_eq(&self, other: &Self) -> bool {
        self.present() == other.present()
            && self.writable() == other.writable()
            && self.user_mode_accessible() == other.user_mode_accessible()
            && self.page_level_write_through() == other.page_level_write_through()
            && self.page_level_cache_disable() == other.page_level_cache_disable()
            && self.phys_addr() == other.phys_addr()
            && self.not_executable() == other.not_executable()
    }
}

impl PageTableEntry2MToTable {
    /// You must guarantee that this is a 2 MiB entry that points to a table.
    fn from_common(entry: PageTableEntryCommon) -> Self {
        Self::new_with_raw_value(entry.raw_value)
    }
}

/// PDE - Table 5-18. Format of a Page-Directory Entry that Maps a 2-MByte Page
#[bitfield(u64, debug)]
#[derive(Default)]
pub struct PageTableEntry2MToPage {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    writable: bool,
    #[bit(2, rw)]
    user_mode_accessible: bool,
    #[bit(3, rw)]
    page_level_write_through: bool,
    #[bit(4, rw)]
    page_level_cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(6, rw)]
    dirty: bool,
    #[bit(7, rw)]
    page_size: bool,
    #[bit(8, rw)]
    global: bool,
    #[bit(12, rw)]
    pat: bool,
    #[bits(21..=51, rw)]
    phys_addr: u31,
    #[bit(63, rw)]
    not_executable: bool,
}

impl PageTableEntry2MToPage {
    /// You must guarantee that this is a 2 MiB entry that points to a page.
    pub fn from_common(common: PageTableEntryCommon) -> Self {
        Self::new_with_raw_value(common.raw_value)
    }
}

impl EntryCommon for PageTableEntry2MToPage {
    fn phys_addr_u64(&self) -> u64 {
        u64::from(self.phys_addr().value()) << (12 + 9)
    }

    fn config_eq(&self, other: &Self) -> bool {
        self.present() == other.present()
            && self.writable() == other.writable()
            && self.user_mode_accessible() == other.user_mode_accessible()
            && self.page_level_write_through() == other.page_level_write_through()
            && self.page_level_cache_disable() == other.page_level_cache_disable()
            && self.pat() == other.pat()
            && self.global() == other.global()
            && self.phys_addr() == other.phys_addr()
            && self.not_executable() == other.not_executable()
    }
}

/// PDPTE - Table 5-17. Format of a Page-Directory-Pointer-Table Entry (PDPTE) that References a Page Directory
#[bitfield(u64, debug)]
#[derive(Default)]
pub struct PageTableEntry1GToTable {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    writable: bool,
    #[bit(2, rw)]
    user_mode_accessible: bool,
    #[bit(3, rw)]
    page_level_write_through: bool,
    #[bit(4, rw)]
    page_level_cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(7, rw)]
    page_size: bool,
    #[bits(12..=51, rw)]
    phys_addr: u40,
    #[bit(63, rw)]
    not_executable: bool,
}

impl PageTableEntry1GToTable {
    /// You must guarantee that this is a 1 GiB entry that points to a table.
    fn from_common(entry: PageTableEntryCommon) -> Self {
        Self::new_with_raw_value(entry.raw_value)
    }
}

impl EntryCommon for PageTableEntry1GToTable {
    fn phys_addr_u64(&self) -> u64 {
        self.phys_addr().value() << 12
    }

    fn config_eq(&self, other: &Self) -> bool {
        self.present() == other.present()
            && self.writable() == other.writable()
            && self.user_mode_accessible() == other.user_mode_accessible()
            && self.page_level_write_through() == other.page_level_write_through()
            && self.page_level_cache_disable() == other.page_level_cache_disable()
            && self.phys_addr() == other.phys_addr()
            && self.not_executable() == other.not_executable()
    }
}

/// PDPTE - Table 5-16. Format of a Page-Directory-Pointer-Table Entry (PDPTE) that Maps a 1-GByte Page
#[bitfield(u64, debug)]
#[derive(Default)]
pub struct PageTableEntry1GToPage {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    writable: bool,
    #[bit(2, rw)]
    user_mode_accessible: bool,
    #[bit(3, rw)]
    page_level_write_through: bool,
    #[bit(4, rw)]
    page_level_cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(6, rw)]
    dirty: bool,
    #[bit(7, rw)]
    page_size: bool,
    #[bit(8, rw)]
    global: bool,
    #[bit(12, rw)]
    pat: bool,
    #[bits(30..=51, rw)]
    phys_addr: u22,
    #[bit(63, rw)]
    not_executable: bool,
}

impl PageTableEntry1GToPage {
    /// You must guarantee that this is a 1 GiB entry that points to a page.
    pub fn from_common(common: PageTableEntryCommon) -> Self {
        Self::new_with_raw_value(common.raw_value)
    }
}

impl EntryCommon for PageTableEntry1GToPage {
    fn phys_addr_u64(&self) -> u64 {
        u64::from(self.phys_addr().value()) << (12 + 9 + 9)
    }

    fn config_eq(&self, other: &Self) -> bool {
        self.present() == other.present()
            && self.writable() == other.writable()
            && self.user_mode_accessible() == other.user_mode_accessible()
            && self.page_level_write_through() == other.page_level_write_through()
            && self.page_level_cache_disable() == other.page_level_cache_disable()
            && self.pat() == other.pat()
            && self.global() == other.global()
            && self.phys_addr() == other.phys_addr()
            && self.not_executable() == other.not_executable()
    }
}

/// PML4E - Table 5-15. Format of a PML4 Entry (PML4E) that References a Page-Directory-Pointer Table
#[bitfield(u64, debug)]
#[derive(Default)]
pub struct PageTableEntry512GToTable {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    writable: bool,
    #[bit(2, rw)]
    user_mode_accessible: bool,
    #[bit(3, rw)]
    page_level_write_through: bool,
    #[bit(4, rw)]
    page_level_cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(7, rw)]
    page_size: bool,
    #[bits(12..=51, rw)]
    phys_addr: u40,
    #[bit(63, rw)]
    not_executable: bool,
}

impl PageTableEntry512GToTable {
    /// You must guarantee that this is a 512 GiB entry that points to a table.
    fn from_common(entry: PageTableEntryCommon) -> Self {
        Self::new_with_raw_value(entry.raw_value)
    }
}

impl EntryCommon for PageTableEntry512GToTable {
    fn phys_addr_u64(&self) -> u64 {
        self.phys_addr().value() << 12
    }

    fn config_eq(&self, other: &Self) -> bool {
        self.present() == other.present()
            && self.writable() == other.writable()
            && self.user_mode_accessible() == other.user_mode_accessible()
            && self.page_level_write_through() == other.page_level_write_through()
            && self.page_level_cache_disable() == other.page_level_cache_disable()
            && self.phys_addr() == other.phys_addr()
            && self.not_executable() == other.not_executable()
    }
}

/// PML5E - Table 5-14. Format of a PML5 Entry (PML5E) that References a PML4 Table
#[bitfield(u64, debug)]
#[derive(Default)]
pub struct PageTableEntry256TToTable {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    writable: bool,
    #[bit(2, rw)]
    user_mode_accessible: bool,
    #[bit(3, rw)]
    page_level_write_through: bool,
    #[bit(4, rw)]
    page_level_cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(7, rw)]
    page_size: bool,
    #[bits(12..=51, rw)]
    phys_addr: u40,
    #[bit(63, rw)]
    not_executable: bool,
}

impl PageTableEntry256TToTable {
    /// You must guarantee that this is a 256 TiB entry that points to a table.
    fn from_common(entry: PageTableEntryCommon) -> Self {
        Self::new_with_raw_value(entry.raw_value)
    }
}

impl EntryCommon for PageTableEntry256TToTable {
    fn phys_addr_u64(&self) -> u64 {
        self.phys_addr().value() << 12
    }

    fn config_eq(&self, other: &Self) -> bool {
        self.present() == other.present()
            && self.writable() == other.writable()
            && self.user_mode_accessible() == other.user_mode_accessible()
            && self.page_level_write_through() == other.page_level_write_through()
            && self.page_level_cache_disable() == other.page_level_cache_disable()
            && self.phys_addr() == other.phys_addr()
            && self.not_executable() == other.not_executable()
    }
}

/// This struct must be a valid page table entry (not for a specific table level) if the present bit is set.
#[bitfield(u64, debug)]
pub struct PageTableEntryCommon {
    #[bit(0, rw)]
    present: bool,
}

impl From<PageTableEntry1GToPage> for PageTableEntryCommon {
    fn from(entry: PageTableEntry1GToPage) -> Self {
        Self::new_with_raw_value(entry.raw_value())
    }
}

impl From<PageTableEntry2MToPage> for PageTableEntryCommon {
    fn from(entry: PageTableEntry2MToPage) -> Self {
        Self::new_with_raw_value(entry.raw_value())
    }
}

impl From<PageTableEntry4KToPage> for PageTableEntryCommon {
    fn from(entry: PageTableEntry4KToPage) -> Self {
        Self::new_with_raw_value(entry.raw_value())
    }
}

pub fn new_leaf_entry(
    mapping_size: LeafMappingSize,
    flags: LeafMappingFlags,
    phys_addr: u64,
) -> PageTableEntryCommon {
    match mapping_size {
        LeafMappingSize::_4K => PageTableEntry4KToPage::default()
            .with_present(true)
            .with_writable(flags.writable)
            .with_user_mode_accessible(flags.user_mode_accessible)
            .with_not_executable(!flags.executable)
            .with_page_level_write_through(flags.pat_index.value() & (1 << 0) != 0)
            .with_page_level_cache_disable(flags.pat_index.value() & (1 << 1) != 0)
            .with_pat(flags.pat_index.value() & (1 << 2) != 0)
            .with_phys_addr(u40::new(phys_addr >> 12))
            .into(),
        LeafMappingSize::_2M => PageTableEntry2MToPage::default()
            .with_present(true)
            .with_page_size(true)
            .with_writable(flags.writable)
            .with_user_mode_accessible(flags.user_mode_accessible)
            .with_not_executable(!flags.executable)
            .with_page_level_write_through(flags.pat_index.value() & (1 << 0) != 0)
            .with_page_level_cache_disable(flags.pat_index.value() & (1 << 1) != 0)
            .with_pat(flags.pat_index.value() & (1 << 2) != 0)
            .with_phys_addr(u31::new((phys_addr >> (12 + 9)).try_into().unwrap()))
            .into(),
        LeafMappingSize::_1G => PageTableEntry1GToPage::default()
            .with_present(true)
            .with_page_size(true)
            .with_writable(flags.writable)
            .with_user_mode_accessible(flags.user_mode_accessible)
            .with_not_executable(!flags.executable)
            .with_page_level_write_through(flags.pat_index.value() & (1 << 0) != 0)
            .with_page_level_cache_disable(flags.pat_index.value() & (1 << 1) != 0)
            .with_pat(flags.pat_index.value() & (1 << 2) != 0)
            .with_phys_addr(u22::new((phys_addr >> (12 + 9 + 9)).try_into().unwrap()))
            .into(),
    }
}

pub fn new_non_leaf_entry(mapping_size: TableMappingSize, table_addr: u64) -> u64 {
    let phys_addr_field = u40::new(table_addr >> 12);
    match mapping_size {
        TableMappingSize::_2M => {
            PageTableEntry2MToTable::default()
                .with_present(true)
                .with_writable(true)
                .with_user_mode_accessible(true)
                .with_not_executable(false)
                .with_phys_addr(phys_addr_field)
                .raw_value
        }
        TableMappingSize::_1G => {
            PageTableEntry1GToTable::default()
                .with_present(true)
                .with_writable(true)
                .with_user_mode_accessible(true)
                .with_not_executable(false)
                .with_phys_addr(phys_addr_field)
                .raw_value
        }
        TableMappingSize::_512G => {
            PageTableEntry512GToTable::default()
                .with_present(true)
                .with_writable(true)
                .with_user_mode_accessible(true)
                .with_not_executable(false)
                .with_phys_addr(phys_addr_field)
                .raw_value
        }
        TableMappingSize::_256T => {
            PageTableEntry256TToTable::default()
                .with_present(true)
                .with_writable(true)
                .with_user_mode_accessible(true)
                .with_not_executable(false)
                .with_phys_addr(phys_addr_field)
                .raw_value
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EntryMappingType {
    Leaf,
    Table,
}

#[derive(Debug, Clone, Copy)]
pub struct EntryMappingInfo {
    pub mapping_type: EntryMappingType,
    pub phys_addr: u64,
}

/// Expected must be a present leaf mapping.
/// Assumes actual is present.
pub fn is_leaf_mapping_and_config_eq(
    mapping_size: LeafMappingSize,
    expected_leaf_entry: PageTableEntryCommon,
    actual_entry: PageTableEntryCommon,
) -> bool {
    match mapping_size {
        LeafMappingSize::_1G => {
            if PageTableEntry2M1GCommon::from_common(actual_entry).is_leaf_mapping() {
                PageTableEntry1GToPage::from_common(expected_leaf_entry)
                    .config_eq(&PageTableEntry1GToPage::from_common(actual_entry))
            } else {
                false
            }
        }
        LeafMappingSize::_2M => {
            if PageTableEntry2M1GCommon::from_common(actual_entry).is_leaf_mapping() {
                PageTableEntry2MToPage::from_common(expected_leaf_entry)
                    .config_eq(&PageTableEntry2MToPage::from_common(actual_entry))
            } else {
                false
            }
        }
        LeafMappingSize::_4K => PageTableEntry4KToPage::from_common(expected_leaf_entry).config_eq(
            &PageTableEntry4KToPage::new_with_raw_value(actual_entry.raw_value),
        ),
    }
}

/// Assumes the entry is present.
pub fn entry_mapping_info(entry_size: EntrySize, entry: PageTableEntryCommon) -> EntryMappingInfo {
    match entry_size {
        EntrySize::_256T => EntryMappingInfo {
            mapping_type: EntryMappingType::Table,
            phys_addr: PageTableEntry256TToTable::from_common(entry).phys_addr_u64(),
        },
        EntrySize::_512G => EntryMappingInfo {
            mapping_type: EntryMappingType::Table,
            phys_addr: PageTableEntry512GToTable::from_common(entry).phys_addr_u64(),
        },
        EntrySize::_1G => {
            if PageTableEntry2M1GCommon::from_common(entry).is_leaf_mapping() {
                EntryMappingInfo {
                    mapping_type: EntryMappingType::Leaf,
                    phys_addr: PageTableEntry1GToPage::from_common(entry).phys_addr_u64(),
                }
            } else {
                EntryMappingInfo {
                    mapping_type: EntryMappingType::Table,
                    phys_addr: PageTableEntry1GToTable::from_common(entry).phys_addr_u64(),
                }
            }
        }
        EntrySize::_2M => {
            if PageTableEntry2M1GCommon::from_common(entry).is_leaf_mapping() {
                EntryMappingInfo {
                    mapping_type: EntryMappingType::Leaf,
                    phys_addr: PageTableEntry2MToPage::from_common(entry).phys_addr_u64(),
                }
            } else {
                EntryMappingInfo {
                    mapping_type: EntryMappingType::Table,
                    phys_addr: PageTableEntry2MToTable::from_common(entry).phys_addr_u64(),
                }
            }
        }
        EntrySize::_4K => EntryMappingInfo {
            mapping_type: EntryMappingType::Leaf,
            phys_addr: PageTableEntry4KToPage::from_common(entry).phys_addr_u64(),
        },
    }
}

// impl PageTableEntry {
//     pub fn address(&self, page_table_level: TableLevel) -> u64 {
//         let shift = match page_table_level {
//             TableLevel::Maps2M => 12,
//             TableLevel::Maps1G => {
//                 if self.page_size() {
//                     12 + 9
//                 } else {
//                     12
//                 }
//             }
//             TableLevel::Maps512G => {
//                 if self.page_size() {
//                     12 + 9 + 9
//                 } else {
//                     12
//                 }
//             }
//             TableLevel::Maps256T => 12,
//             TableLevel::Maps128P => 12,
//         };
//         ((self.raw_value() >> shift) & ((1 << 52) - 1)) << shift
//     }

//     pub fn eq_configuration(&self, other: &Self, page_table_level: TableLevel) -> bool {
//         self.present() == other.present()
//             && self.address(page_table_level) == other.address(page_table_level)
//             && self.writable() == other.writable()
//             && self.user_mode_accessible() == other.user_mode_accessible()
//             && self.page_level_write_through() == other.page_level_write_through()
//             && self.page_level_cache_disable() == other.page_level_cache_disable()
//             && self.not_executable() == other.not_executable()
//     }
// }
