use crate::{DYNAMIC_VIRT_ADDR, DYNAMIC_VIRT_LEN};

/// For now it's just a bump allocator.
#[derive(Debug, Default)]
pub struct Vmm {
    position: u64,
}

impl Vmm {
    pub fn alloc(&mut self, size: u64, align: u64) -> Option<u64> {
        let aligned_position = self.position.next_multiple_of(align);
        let potential_end = aligned_position + size;
        if potential_end <= DYNAMIC_VIRT_LEN {
            self.position = potential_end;
            Some(DYNAMIC_VIRT_ADDR + aligned_position)
        } else {
            None
        }
    }
}
