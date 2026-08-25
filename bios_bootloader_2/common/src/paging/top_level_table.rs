use core::ptr::NonNull;

use crate::paging::{
    LeafMapping, LeafMappingSize, PageTable, ScratchPageTable, TableMappingSize,
    TableMappingVirtAddr,
    page_table_entry::{
        EntryMappingInfo, EntryMappingType, PageTableEntryCommon, entry_mapping_info,
        is_leaf_mapping_and_config_eq, new_leaf_entry, new_non_leaf_entry,
    },
    table_level::TableLevel,
    top_level::TopLevel,
    virt_addr::VirtAddr,
};

/// Owns top level page table and all page tables below it. Could be with 4-level or 5-level paging, so in total it could map 256 TiB or 128 PiB of virtual memory.
#[derive(Debug)]
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
        table_to_attach_addr: u64,
        mut scratch_tables: impl Iterator<Item = ScratchPageTable>,
    ) -> Result<(), AttachError> {
        let mut table_level = TableLevel::from(self.level);
        let mut table_addr = self.addr;
        loop {
            let mut table = NonNull::new((table_addr + self.offset) as *mut PageTable).unwrap();
            // Safety: table is valid and mapped
            let table = unsafe { table.as_mut() };
            let entry_index = mapping.addr.index_in_table(table_level);
            let raw_entry = &mut table[entry_index];
            let entry_table_mapping_size = TableMappingSize::try_from(table_level.entry_size()).expect("we would've hit the branch where the mapping size is the size of the mapping that we want to attach");
            if entry_table_mapping_size == mapping.size {
                if PageTableEntryCommon::new_with_raw_value(*raw_entry).present() {
                    return Err(AttachError::AlreadyMapped);
                }
                *raw_entry = new_non_leaf_entry(mapping.size, table_to_attach_addr);
                break;
            } else {
                let read_raw_entry = PageTableEntryCommon::new_with_raw_value(*raw_entry);
                if read_raw_entry.present() {
                    let EntryMappingInfo {
                        mapping_type: EntryMappingType::Table,
                        phys_addr,
                    } = entry_mapping_info(table_level.entry_size(), read_raw_entry)
                    else {
                        return Err(AttachError::AlreadyMapped);
                    };
                    table_addr = phys_addr;
                    table_level = table_level.child().unwrap();
                } else {
                    // Create a child table
                    let child_table = scratch_tables
                        .next()
                        .ok_or(AttachError::OutOfScratchTables)?;
                    *raw_entry = new_non_leaf_entry(entry_table_mapping_size, child_table.addr);
                    table_addr = child_table.addr;
                    table_level = table_level.child().unwrap();
                }
            }
        }
        Ok(())
    }

    unsafe fn map_leaf_internal(
        &mut self,
        mapping: LeafMapping,
        scratch_tables: &mut impl Iterator<Item = ScratchPageTable>,
        must_create_new: bool,
    ) -> Result<(), MapError> {
        let mut table_level = TableLevel::from(self.level);
        let mut table_addr = self.addr;
        loop {
            let mut table = NonNull::new((table_addr + self.offset) as *mut PageTable).unwrap();
            // Safety: table is valid and mapped
            let table = unsafe { table.as_mut() };
            let entry_index =
                VirtAddr::new_with_raw_value(mapping.virt_addr).index_in_table(table_level);
            let raw_entry = &mut table[entry_index];
            if LeafMappingSize::try_from(table_level.entry_size()) == Ok(mapping.size) {
                let expected_entry = new_leaf_entry(mapping.size, mapping.flags, mapping.phys_addr);
                let current_raw_entry = PageTableEntryCommon::new_with_raw_value(*raw_entry);
                if current_raw_entry.present() {
                    if must_create_new
                        || !is_leaf_mapping_and_config_eq(
                            mapping.size,
                            expected_entry,
                            current_raw_entry,
                        )
                    {
                        // panic!(
                        //     "mapping: {mapping:X?}. expected: {:#X}. actual: {:#X}. must create new: {must_create_new}.",
                        //     expected_entry.raw_value(),
                        //     current_raw_entry.raw_value()
                        // );
                        return Err(MapError::AlreadyMapped);
                    }
                } else {
                    *raw_entry = expected_entry.raw_value();
                    log::trace!("{table_level:?} created leaf entry, raw: {raw_entry:#X}.");
                }
                break;
            } else {
                let read_raw_entry = PageTableEntryCommon::new_with_raw_value(*raw_entry);
                if read_raw_entry.present() {
                    let EntryMappingInfo {
                        mapping_type: EntryMappingType::Table,
                        phys_addr,
                    } = entry_mapping_info(table_level.entry_size(), read_raw_entry)
                    else {
                        return Err(MapError::AlreadyMapped);
                    };
                    table_addr = phys_addr;
                    table_level = table_level.child().unwrap();
                } else {
                    let child_table = scratch_tables.next().ok_or(MapError::OutOfScratchTables)?;

                    *raw_entry = new_non_leaf_entry(
                        table_level.entry_size().try_into().unwrap(),
                        child_table.addr,
                    );
                    log::trace!(
                        "table level: {table_level:?}[{entry_index}], created raw entry {raw_entry:#X}."
                    );
                    table_addr = child_table.addr;
                    table_level = table_level.child().unwrap();
                }
            }
        }
        Ok(())
    }

    pub unsafe fn map_leaf(
        &mut self,
        mapping: LeafMapping,
        scratch_tables: &mut impl Iterator<Item = ScratchPageTable>,
    ) -> Result<(), MapError> {
        unsafe { self.map_leaf_internal(mapping, scratch_tables, true) }
    }

    pub unsafe fn ensure_mapped_leaf(
        &mut self,
        mapping: LeafMapping,
        scratch_tables: &mut impl Iterator<Item = ScratchPageTable>,
    ) -> Result<(), EnsureMappedError> {
        unsafe { self.map_leaf_internal(mapping, scratch_tables, false) }.map_err(|e| match e {
            MapError::AlreadyMapped => EnsureMappedError::AlreadyMappedDifferently,
            MapError::OutOfScratchTables => EnsureMappedError::OutOfScratchTables,
        })
    }
}

#[derive(Debug)]
pub enum AttachError {
    AlreadyMapped,
    OutOfScratchTables,
}

#[derive(Debug)]
pub enum MapError {
    AlreadyMapped,
    OutOfScratchTables,
}

#[derive(Debug)]
pub enum EnsureMappedError {
    AlreadyMappedDifferently,
    OutOfScratchTables,
}
