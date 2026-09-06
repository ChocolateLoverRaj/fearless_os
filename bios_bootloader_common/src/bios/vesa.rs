use core::ptr::{NonNull, addr_of};

use arbitrary_int::u9;
use bitbybit::bitfield;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout,
    little_endian::{U16, U32},
};

use crate::bios::{BiosFns, RealModeAddr};

/// count of how many bytes to print
pub(super) type PrintCharFn = extern "C" fn(u16);

impl BiosFns {
    pub fn print(&self, str: &[u8]) {
        for chunk in str.chunks(size_of_val(&self.table().buffer)) {
            self.table().buffer[..chunk.len()].copy_from_slice(chunk);
            (self.table().int_10)(chunk.len().try_into().unwrap())
        }
    }
}

pub(super) type VesaGetInfoFn = extern "C" fn() -> u16;

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

enum ModeListPointer {
    WithinModeInfo { offset: usize },
    Outside(&'static [U16]),
}

pub struct VbeInfoWithList {
    info: VbeInfoBlock,
    list: ModeListPointer,
}

impl VbeInfoWithList {
    pub fn info(&self) -> &VbeInfoBlock {
        &self.info
    }

    pub fn mode_list(&self) -> VideoModesList<'_> {
        VideoModesList {
            list: match &self.list {
                ModeListPointer::WithinModeInfo { offset } => {
                    let list = <[U16]>::ref_from_prefix(&self.info().as_bytes()[*offset..])
                        .unwrap()
                        .0;
                    let end_exclusive =
                        list.iter().position(|item| item.get() == u16::MAX).unwrap();
                    &list[..end_exclusive]
                }
                ModeListPointer::Outside(list) => list,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VideoModesList<'a> {
    list: &'a [U16],
}

impl<'a> IntoIterator for VideoModesList<'a> {
    type Item = u9;
    type IntoIter = VideoModesIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        VideoModesIterator {
            list: self.list,
            index: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VideoModesIterator<'a> {
    list: &'a [U16],
    index: usize,
}

impl Iterator for VideoModesIterator<'_> {
    type Item = u9;

    fn next(&mut self) -> Option<Self::Item> {
        let mode = u9::new(self.list.get(self.index)?.get());
        self.index += 1;
        Some(mode)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VesaError {
    InvalidAx(u16),
}

impl BiosFns {
    pub fn get_vbe_info(&self) -> Result<VbeInfoWithList, VesaError> {
        self.table().buffer[..4].copy_from_slice(b"VBE2");
        let ax = (self.table().vesa_get_info)();
        if ax != 0x004f {
            return Err(VesaError::InvalidAx(ax));
        }
        let vbe_info_block = VbeInfoBlock::read_from_prefix(&self.table().buffer)
            .unwrap()
            .0;
        let list_ptr = usize::try_from(u32::from(RealModeAddr {
            offset: vbe_info_block.video_modes_offset,
            segment: vbe_info_block.video_modes_segment,
        }))
        .unwrap();
        let buffer_addr = addr_of!(self.table().vesa_get_info).addr();
        let buffer_range = buffer_addr..buffer_addr + size_of::<VbeInfoBlock>();
        let vbe_info_with_list = VbeInfoWithList {
            info: vbe_info_block,
            list: if buffer_range.contains(&list_ptr) {
                ModeListPointer::WithinModeInfo {
                    offset: list_ptr - buffer_addr,
                }
            } else {
                let mut len = 0;
                loop {
                    let ptr =
                        NonNull::new((list_ptr + len * size_of::<U16>()) as *mut U16).unwrap();
                    let mode = unsafe { ptr.read_unaligned() }.get();
                    if mode == u16::MAX {
                        break;
                    }
                    len += 1;
                }
                let slice =
                    NonNull::slice_from_raw_parts(NonNull::new(list_ptr as *mut U16).unwrap(), len);
                ModeListPointer::Outside(unsafe { slice.as_ref() })
            },
        };
        Ok(vbe_info_with_list)
    }
}

// pub struct VideoModesIterator {
//     pointer: NonNull<u16>,
// }

// impl VideoModesIterator {
//     pub unsafe fn new(vbe_info_block: &VbeInfoBlock) -> Self {
//         Self {
//             pointer: NonNull::new(
//                 usize::try_from(
//                     u32::from(vbe_info_block.video_modes_segment) * 16
//                         + u32::from(vbe_info_block.video_modes_offset),
//                 )
//                 .unwrap() as *mut _,
//             )
//             .unwrap(),
//         }
//     }
// }
// impl Iterator for VideoModesIterator {
//     type Item = u16;

//     fn next(&mut self) -> Option<Self::Item> {
//         let video_mode = unsafe { self.pointer.read_unaligned() };
//         if video_mode == 0xffff {
//             return None;
//         }
//         self.pointer =
//             NonNull::new((self.pointer.addr().get() + size_of::<u16>()) as *mut _).unwrap();
//         Some(video_mode)
//     }
// }

/// mode
pub(super) type VesaGetModeFn = extern "C" fn(u16) -> u16;

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct ModeInfo {
    pub mode_attributs: U16,
    pub win_a_attributes: u8,
    pub win_b_attributes: u8,
    pub win_granularity: U16,
    pub win_size: U16,
    pub win_a_segment: U16,
    pub win_b_segment: U16,
    pub win_func_ptr: U32,
    pub bytes_per_scan_line: U16,
    // Mandatory info for VBE 1.2 and above
    pub x_resolution: U16,
    pub y_resolution: U16,
    pub x_char_size: u8,
    pub y_char_size: u8,
    pub number_of_planes: u8,
    pub bits_per_pixel: u8,
    pub number_of_banks: u8,
    pub memory_model: u8,
    /// In KiB
    pub bank_size: u8,
    pub number_of_image_pages: u8,
    pub _reserved: u8,
    pub red_mask_size: u8,
    pub red_field_position: u8,
    pub green_mask_size: u8,
    pub green_field_position: u8,
    pub blue_mask_size: u8,
    pub blue_field_position: u8,
    pub rsvd_mask_size: u8,
    pub rsvd_field_position: u8,
    pub direct_color_mode_info: u8,

    // Mandatory info for VBE 2.0 and above
    pub phys_base_ptr: U32,
    pub _reserved_1: U32,
    pub _reserved_2: U16,

    // Mandatory info for VBE 3.0 and above
    pub lin_bytes_per_scan: U16,
    pub bnk_number_of_image_pages: u8,
    pub lin_number_of_image_pages: u8,
    pub lin_red_mask_size: u8,
    pub lin_red_field_position: u8,
    pub lin_green_mask_size: u8,
    pub lin_green_field_position: u8,
    pub lin_blue_mask_size: u8,
    pub lin_blue_field_position: u8,
    pub lin_rsvd_mask_size: u8,
    pub lin_rsvd_field_position: u8,
    pub max_pixel_clock: U32,

    pub _reserved_3: [u8; 189],
}

#[bitfield(u16)]
pub struct VesaModeAttributes {
    #[bit(0, rw)]
    supported_in_hardware: bool,
    #[bit(2, rw)]
    tty_output_functions_supported: bool,
    /// false -> text mode
    /// true -> graphics mode
    #[bit(4, rw)]
    mode_type: bool,
    #[bit(7, rw)]
    linear_frame_buffer_mode_available: bool,
}

impl BiosFns {
    pub fn vesa_get_mode_info(&self, mode: u9) -> Result<ModeInfo, VesaError> {
        let ax = (self.table().vesa_get_mode)(mode.value());
        if ax != 0x004f {
            return Err(VesaError::InvalidAx(ax));
        }
        Ok(ModeInfo::read_from_prefix(&self.table().buffer).unwrap().0)
    }
}

/// mode
pub(super) type VesaSetModeFn = extern "C" fn(u16) -> u16;

#[bitfield(u16)]
struct SetVbeModeBx {
    #[bits(0..=8, rw)]
    mode_number: u9,
    #[bit(11, rw)]
    refresh_rate: bool,
    #[bit(14, rw)]
    linear: bool,
    #[bit(15, rw)]
    dont_clear: bool,
}

impl BiosFns {
    pub fn vesa_set_mode(&self, mode: u9, clear: bool) -> Result<(), VesaError> {
        let bx = SetVbeModeBx::ZERO
            .with_mode_number(mode)
            .with_refresh_rate(false)
            .with_linear(true)
            .with_dont_clear(!clear)
            .raw_value();
        let ax = (self.table().vesa_set_mode)(bx);
        if ax != 0x004f {
            return Err(VesaError::InvalidAx(ax));
        }
        Ok(())
    }
}
