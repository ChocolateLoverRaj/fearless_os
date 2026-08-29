use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::bios::BiosFns;

/// count of how many bytes to print
pub(super) type PrintCharFn = fn(u16);
pub(super) type VesaGetInfoFn = fn() -> u16;

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct VbeInfoBlock {
    pub vbe_signature: [u8; 4],
    pub vbe_version: [u8; 2],
    pub oem_str_offset: u16,
    pub oem_str_segment: u16,
    pub capabilities: [u8; 4],
    pub video_modes_offset: u16,
    pub video_modes_segment: u16,
    /// # of 64 KiB blocks
    pub total_memory: u16,
    pub reserved: [u8; 492],
}

#[derive(Debug, Clone, Copy)]
pub enum VesaGetModeInfoErr {
    InvalidAx(u16),
}

impl BiosFns {
    pub fn print(&self, str: &[u8]) {
        for chunk in str.chunks(size_of_val(&self.table().buffer)) {
            self.table().buffer[..chunk.len()].copy_from_slice(chunk);
            (self.table().int_10)(chunk.len().try_into().unwrap())
        }
    }

    pub fn get_vbe_info(&self) -> Result<VbeInfoBlock, VesaGetModeInfoErr> {
        let ax = (self.table().vesa_get_info)();
        log::info!("ax: {:#x}", ax);
        if ax != 0x004f {
            return Err(VesaGetModeInfoErr::InvalidAx(ax));
        }
        Ok(VbeInfoBlock::read_from_prefix(&self.table().buffer)
            .unwrap()
            .0)
    }
}
