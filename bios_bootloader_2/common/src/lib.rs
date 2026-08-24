#![no_std]
pub mod big_stage_api;
pub mod bios;
pub mod logger;
pub mod paging;
pub mod pat;
pub mod writer_with_cr;

pub const STACK_TOP: u16 = 0x7C00;
pub const PARTITION_SECTOR_0: u16 = 0x7C00;
pub const PAGE_TABLE_256T: u16 = 0x8000;
pub const PAGE_TABLE_512G: u16 = 0x9000;
pub const PAGE_TABLE_1G: u16 = 0xA000;
pub const SECTOR_1: u16 = 0xB000;
pub const BIG_STAGE_LOAD_ADDR: u64 = 0xFFFFFFFF80000000;
pub const OFFSET_MAP_VIRT_ADDR: u64 = 0xFFFFC00000000000;
