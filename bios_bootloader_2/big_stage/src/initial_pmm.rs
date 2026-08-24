use core::ops::Range;

/// A Physical Memory Manager (PMM) that doesn't need any initial PMM.
/// This is implemented as a bump allocator.
/// It can be used to intialize a better PMM.
pub struct InitialPmm<'a> {
    original_free_memory: &'a [Range<u64>],
    current_index: usize,
    current_offset: u64,
}

impl<'a> InitialPmm<'a> {
    pub fn new(free_memory: &'a [Range<u64>]) -> Self {
        Self {
            original_free_memory: free_memory,
            current_index: 0,
            current_offset: 0,
        }
    }

    /// Size must be a multiple of 4 KiB since that's the smallest granularity we use for phys mem.
    pub fn allocate(&mut self, size: u64, align: u64) -> Option<u64> {
        loop {
            let entry = self.original_free_memory.get(self.current_index)?;
            let free_start = entry.start + self.current_offset;
            let potential_aligned_start = free_start.next_multiple_of(align);
            let potential_end = potential_aligned_start + size;
            if potential_end <= entry.end {
                self.current_offset = potential_end - entry.start;
                log::info!("PMM: {potential_aligned_start:#X} {size:#X}");
                break Some(potential_aligned_start);
            }
            self.current_index += 1;
            self.current_offset = 0;
        }
    }
}
