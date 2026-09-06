use nodit::{Interval, NoditMap};
use x86_64::{
    PhysAddr,
    structures::paging::{FrameAllocator, PageSize, PhysFrame, Size4KiB},
};

/// Note that there are other memory types (such as ACPI memory) that are not included here
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MemoryType {
    Free,
    Used,
}

#[derive(Debug)]
pub struct PhysicalMemory {
    pub map: NoditMap<u64, Interval<u64>, MemoryType>,
}

impl PhysicalMemory {
    pub fn allocate_frame_with_type(
        &mut self,
        size: u64,
        align: u64,
        memory_type: MemoryType,
    ) -> Option<u64> {
        let aligned_start = self.map.iter().find_map(|(interval, memory_type)| {
            if let MemoryType::Free = memory_type {
                let aligned_start = interval.start().next_multiple_of(align);
                let required_end = aligned_start + size;
                if required_end <= *interval.end() {
                    Some(aligned_start)
                } else {
                    None
                }
            } else {
                None
            }
        })?;
        let range = aligned_start..aligned_start + size;
        let _ = self.map.cut(&Interval::from(range.clone()));
        self.map
            .insert_merge_touching_if_values_equal(range.into(), memory_type)
            .unwrap();
        Some(aligned_start)
    }

    pub fn get_kernel_frame_allocator(&mut self) -> PhysicalMemoryFrameAllocator<'_> {
        PhysicalMemoryFrameAllocator {
            physical_memory: self,
            memory_type: MemoryType::Used,
        }
    }
}

pub struct PhysicalMemoryFrameAllocator<'a> {
    physical_memory: &'a mut PhysicalMemory,
    memory_type: MemoryType,
}

impl PhysicalMemoryFrameAllocator<'_> {
    // pub fn allocate_4kib_frame(&mut self) -> Option<PhysFrame> {
    //     let frame = self
    //         .physical_memory
    //         .allocate_frame_with_type(PageSize::_4KiB, self.memory_type)?;
    //     let frame = PhysFrame::from_start_address(frame.start_addr()).unwrap();
    //     Some(frame)
    // }
}

unsafe impl FrameAllocator<Size4KiB> for PhysicalMemoryFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let addr = self.physical_memory.allocate_frame_with_type(
            Size4KiB::SIZE,
            Size4KiB::SIZE,
            self.memory_type,
        )?;
        Some(PhysFrame::from_start_address(PhysAddr::new(addr)).unwrap())
    }
}
