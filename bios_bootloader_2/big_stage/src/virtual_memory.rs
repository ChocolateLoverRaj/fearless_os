use core::ptr::addr_of;

use nodit::{Interval, NoditSet};
use x86_64::registers::control::Cr3;

use crate::__start;

#[derive(Debug)]
pub struct VirtualMemory {
    #[allow(unused)]
    pub(super) set: NoditSet<u64, Interval<u64>>,
}

impl VirtualMemory {
    pub fn alloc(&mut self, size: u64, align: u64) -> Option<u64> {
        let aligned_start = self
            .set
            .gaps_trimmed(&Interval::from(
                addr_of!(__start) as u64..=0xFFFFFFFFFFFFFFFF,
            ))
            .find_map(|interval| {
                let aligned_start = interval.start().next_multiple_of(align);
                let required_end = aligned_start + size;
                if required_end <= *interval.end() {
                    Some(aligned_start)
                } else {
                    None
                }
            })?;
        let range = aligned_start..aligned_start + size;
        self.set.insert_merge_touching(range.into()).unwrap();
        Some(aligned_start)
    }

    pub fn table(&mut self) {
        let page_table_256_t = Cr3::read().0;
    }
}
