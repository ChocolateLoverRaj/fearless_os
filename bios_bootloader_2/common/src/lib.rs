#![no_std]
pub const STACK_TOP: u16 = 0x7C00;
pub const PARTITION_SECTOR_0: u16 = 0x7C00;
pub const PAGE_TABLE_256T: u16 = 0x8000;
pub const PAGE_TABLE_512G: u16 = 0x9000;
pub const PAGE_TABLE_1G: u16 = 0xA000;
pub const SECTOR_1: u16 = 0xB000;
pub const BIG_STAGE_ADDR: u64 = 0xFFFFFFFF80000000;
