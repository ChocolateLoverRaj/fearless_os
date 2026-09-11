use core::{cmp::max, ops::Range};

/// Entries must be sorted.
pub trait InitialFreeMem: Iterator<Item = Range<u64>> + Send + Sync {
    fn phys_end_addr(&self) -> u64;
}

/// A Physical Memory Manager (PMM) that doesn't need any initial PMM.
/// This is implemented as a bump allocator.
/// It can be used to intialize a better PMM.
pub struct InitialPmm<'a> {
    original_free_memory: &'a mut dyn InitialFreeMem,
    current_range: Option<Range<u64>>,
    current_offset: u64,
}

impl<'a> InitialPmm<'a> {
    pub fn new(free_memory: &'a mut dyn InitialFreeMem) -> Self {
        Self {
            current_range: free_memory.next(),
            original_free_memory: free_memory,
            current_offset: 0,
        }
    }

    /// Size must be a multiple of 4 KiB since that's the smallest granularity we use for phys mem.
    pub fn allocate(&mut self, size: u64, align: u64) -> Option<u64> {
        loop {
            let entry = self.current_range.as_ref()?;
            let free_start = entry.start + self.current_offset;
            let potential_aligned_non_zero_start = max(free_start, 1).next_multiple_of(align);
            let potential_end = potential_aligned_non_zero_start + size;
            if potential_end <= entry.end {
                self.current_offset = potential_end - entry.start;
                log::trace!("PMM: {potential_aligned_non_zero_start:#X} {size:#X}");
                break Some(potential_aligned_non_zero_start);
            }
            self.current_range = self.original_free_memory.next();
            self.current_offset = 0;
        }
    }
}
