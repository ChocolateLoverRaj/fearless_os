mod leaf_mapping;
mod page_table;
mod page_table_entry;
mod scratch_page_table;
mod table_mapping_size;
mod table_mapping_virt_addr;
mod virt_addr;

use core::ptr::NonNull;

use arbitrary_int::u9;
use raw_cpuid::CpuId;

use crate::paging::page_table_entry::PageTableEntry;

pub use leaf_mapping::{LeafMapping, LeafMappingSize};
pub use page_table::PageTable;
pub use scratch_page_table::ScratchPageTable;
pub use table_mapping_size::TableMappingSize;
pub use table_mapping_virt_addr::TableMappingVirtAddr;

/// Owns top level page table and all page tables below it. Could be with 4-level or 5-level paging, so in total it could map 256 TiB or 128 PiB of virtual memory.
pub struct TopLevelPageTable {
    /// Virt addr that points to 0x0 in physical memory.
    /// Every physical memory region passed to this struct must be mapped.
    offset: u64,
    addr: u64,
    level: TopLevel,
}

impl TopLevelPageTable {
    /// # Safety
    /// - Every physical memory region passed to this struct must be accessible at the virtual address starting from this offset.
    /// - The phys addr of the top level table must be free to use by this struct (4 KiB size and align). It must be a valid page table or zeroed (valid empty page table).
    pub unsafe fn new(offset: u64, addr: u64, level: TopLevel) -> Self {
        Self {
            offset,
            addr,
            level,
        }
    }

    /// Creates a mapping that points to an existing page table. For example, you could use it to map the lower 256 TiB in a 128 PiB page table to an existing 256 TiB page table. Uses up to 3 scratch tables to create child page tables as needed.
    ///
    ///  # Safety
    /// - The table you are attaching must be valid
    /// - Scratch tables must be valid
    pub unsafe fn attach_existing_page_table(
        &mut self,
        mapping: TableMappingVirtAddr,
        table_addr: u64,
        mut scratch_tables: impl Iterator<Item = ScratchPageTable>,
    ) -> Result<(), AttachError> {
        let mut level = TableLevel::from(self.level);
        let mut current_table_addr = self.addr;
        loop {
            let mut table =
                NonNull::new((current_table_addr + self.offset) as *mut PageTable).unwrap();
            // Safety: table is valid and mapped
            let table = unsafe { table.as_mut() };
            let entry_index = mapping.addr.index_in_table(level);
            let raw_entry = &mut table[entry_index];
            if level.table_mapping_size().unwrap() == mapping.size {
                *raw_entry = PageTableEntry::new_with_raw_value(table_addr)
                    .with_writable(true)
                    .with_user_mode_accessible(true)
                    .with_not_executable(false)
                    .raw_value();
                break;
            } else {
                let entry = PageTableEntry::new_with_raw_value(*raw_entry);
                if entry.present() {
                    if !entry.page_size() {
                        return Err(AttachError::AlreadyMapped {
                            table: level,
                            entry_index,
                        });
                    }
                    current_table_addr = entry.address(level);
                } else {
                    // Create a child table
                    let child_table = scratch_tables
                        .next()
                        .ok_or(AttachError::OutOfScratchTables)?;
                    *raw_entry = PageTableEntry::new_with_raw_value(child_table.addr)
                        .with_present(true)
                        .with_writable(true)
                        .with_user_mode_accessible(true)
                        .with_not_executable(false)
                        .raw_value();
                    current_table_addr = child_table.addr;
                }
                level = level.child().unwrap();
            }
        }
        Ok(())
    }

    pub unsafe fn map_leaf(
        &mut self,
        mapping: LeafMapping,
        mut scratch_tables: impl Iterator<Item = ScratchPageTable>,
    ) -> Result<(), MapError> {
        let mut level = TableLevel::from(self.level);
        let mut current_table_addr = self.addr;
        loop {
            let mut table =
                NonNull::new((current_table_addr + self.offset) as *mut PageTable).unwrap();
            // Safety: table is valid and mapped
            let table = unsafe { table.as_mut() };
            let entry_index = mapping.virt_addr.index_in_table(level);
            let raw_entry = &mut table[entry_index];
            if level.mapping_size() == Some(mapping.size) {
                *raw_entry = {
                    let mut entry = PageTableEntry::new_with_raw_value(0)
                        .with_present(true)
                        .with_writable(true)
                        .with_user_mode_accessible(true)
                        .with_not_executable(false);
                    mapping.make_entry(&mut entry);
                    entry.raw_value()
                };
                break;
            } else {
                let entry = PageTableEntry::new_with_raw_value(*raw_entry);
                if entry.present() {
                    if entry.page_size() {
                        return Err(MapError::AlreadyMapped {
                            table: level,
                            entry_index: entry_index as u9,
                        });
                    } else {
                        current_table_addr = entry.address(level);
                        if current_table_addr == 0 {
                            panic!("Entry: {:#X}.", entry.raw_value());
                        }
                    }
                } else {
                    let child_table = scratch_tables.next().ok_or(MapError::OutOfScratchTables)?;
                    *raw_entry = PageTableEntry::new_with_raw_value(child_table.addr)
                        .with_present(true)
                        .with_writable(true)
                        .with_user_mode_accessible(true)
                        .with_not_executable(false)
                        .raw_value();
                    current_table_addr = child_table.addr;
                }
                level = level.child().unwrap();
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum AttachError {
    AlreadyMapped { table: TableLevel, entry_index: u9 },
    OutOfScratchTables,
}

#[derive(Debug)]
pub enum MapError {
    AlreadyMapped { table: TableLevel, entry_index: u9 },
    OutOfScratchTables,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopLevel {
    /// Top table when using 5-level paging. Maps 128 PiB.
    Maps128P,
    /// Top table when using 4-level paging. Maps 256 TiB.
    Maps256T,
}

impl TopLevel {
    pub fn max_supported() -> Self {
        if CpuId::new()
            .get_extended_feature_info()
            .is_some_and(|info| info.has_la57())
        {
            TopLevel::Maps128P
        } else {
            TopLevel::Maps256T
        }
    }
}

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
    pub fn child(&self) -> Option<Self> {
        match self {
            Self::Maps128P => Some(Self::Maps256T),
            Self::Maps256T => Some(Self::Maps512G),
            Self::Maps512G => Some(Self::Maps1G),
            Self::Maps1G => Some(Self::Maps2M),
            Self::Maps2M => None,
        }
    }

    pub fn table_mapping_size(&self) -> Option<TableMappingSize> {
        match self {
            Self::Maps128P => Some(TableMappingSize::_256T),
            Self::Maps256T => Some(TableMappingSize::_512G),
            Self::Maps512G => Some(TableMappingSize::_1G),
            Self::Maps1G => Some(TableMappingSize::_2M),
            Self::Maps2M => None,
        }
    }

    pub fn mapping_size(&self) -> Option<LeafMappingSize> {
        match self {
            Self::Maps128P => None,
            Self::Maps256T => None,
            Self::Maps512G => Some(LeafMappingSize::_1G),
            Self::Maps1G => Some(LeafMappingSize::_2M),
            Self::Maps2M => Some(LeafMappingSize::_4K),
        }
    }
}
