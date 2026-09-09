#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtMemRange {
    pub addr: u64,
    pub len: u64,
}

/// For now it's just a bump allocator.
#[derive(Debug)]
pub struct Vmm {
    info: VirtMemRange,
    position: u64,
}

impl Vmm {
    pub fn new(dynamic_virt_info: VirtMemRange) -> Self {
        Self {
            info: dynamic_virt_info,
            position: 0,
        }
    }

    pub fn alloc(&mut self, size: u64, align: u64) -> Option<u64> {
        let aligned_position = self.position.next_multiple_of(align);
        let potential_end = aligned_position + size;
        if potential_end <= self.info.len {
            self.position = potential_end;
            Some(self.info.addr + aligned_position)
        } else {
            None
        }
    }
}
