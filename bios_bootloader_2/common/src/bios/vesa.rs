use zerocopy::{FromBytes, Immutable, IntoBytes};

use crate::bios::BiosFns;

pub(super) type PrintCharFn = fn(u8);
pub(super) type VesaGetInfoFn = fn() -> u16;

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable)]
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
    pub fn print_char(&self, byte: u8) {
        (self.table().int_10)(byte)
    }

    pub fn get_vbe_info(&self) -> Result<VbeInfoBlock, VesaGetModeInfoErr> {
        let ax = (self.table().vesa_get_info)();
        log::info!("ax: {:#x}", ax);
        if ax != 0x004f {
            return Err(VesaGetModeInfoErr::InvalidAx(ax));
        }
        Ok(self.table().vbe_info_buffer)
    }
}
