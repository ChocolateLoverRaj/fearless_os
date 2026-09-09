#![no_std]
#[cfg(feature = "alloc")]
extern crate alloc;

pub mod config;
pub mod frame_buffer;
pub mod frame_buffer_embedded_graphics;
pub mod frame_buffer_info;
pub mod frame_buffer_log_target;
pub mod frame_buffer_writer;
pub mod initial_pmm;
pub mod log_color;
pub mod log_target;
pub mod logger;
pub mod memory;
pub mod nop_log_target;
pub mod paging;
pub mod pat;
pub mod rgb_pixel_info;
pub mod scratch_tables;
pub mod vmm;
