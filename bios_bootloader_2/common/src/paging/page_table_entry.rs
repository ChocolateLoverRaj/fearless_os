use bitbybit::bitfield;

use crate::paging::TableLevel;

#[bitfield(u64, debug)]
pub struct PageTableEntry {
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
    #[bit(63, rw)]
    not_executable: bool,
}

impl PageTableEntry {
    pub fn address(&self, page_table_level: TableLevel) -> u64 {
        let shift = match page_table_level {
            TableLevel::Maps2M => 12,
            TableLevel::Maps1G => {
                if self.page_size() {
                    12 + 9
                } else {
                    12
                }
            }
            TableLevel::Maps512G => {
                if self.page_size() {
                    12 + 9 + 9
                } else {
                    12
                }
            }
            TableLevel::Maps256T => 12,
            TableLevel::Maps128P => 12,
        };
        ((self.raw_value() >> shift) & ((1 << 52) - 1)) << shift
    }
}
