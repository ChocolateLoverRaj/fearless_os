use arbitrary_int::{u9, u12};
use bitbybit::bitfield;

use crate::paging::TableLevel;

#[bitfield(u64, debug)]
pub struct VirtAddr {
    #[bits(0..=11, rw)]
    offset: u12,
    #[bits(12..=20, rw)]
    index_in_2m: u9,
    #[bits(21..=29, rw)]
    index_in_1g: u9,
    #[bits(30..=38, rw)]
    index_in_512g: u9,
    #[bits(39..=47, rw)]
    index_in_256t: u9,
    /// Only exists if 5-level paging is enabled.
    #[bits(48..=56, rw)]
    index_in_128p: u9,
}

impl VirtAddr {
    pub fn index_in_table(&self, table: TableLevel) -> u9 {
        match table {
            TableLevel::Maps128P => self.index_in_128p(),
            TableLevel::Maps256T => self.index_in_256t(),
            TableLevel::Maps512G => self.index_in_512g(),
            TableLevel::Maps1G => self.index_in_1g(),
            TableLevel::Maps2M => self.index_in_2m(),
        }
    }
}
