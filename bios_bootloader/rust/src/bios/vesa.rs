use core::{mem, ops::Div, ptr};

use arbitrary_int::u9;
use bitbybit::bitfield;
use zerocopy::{
    FromBytes, Immutable, IntoBytes,
    little_endian::{U16, U32},
    transmute_mut,
};

pub struct VesaGetControllerInfoPtr(u16);

#[derive(Debug, Clone, Copy)]
pub enum VesaGetControllerInfoError {
    InvalidAx(u16),
    InvalidSignature([u8; 4]),
}

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

impl VesaGetControllerInfoPtr {
    pub fn call<'a>(
        &self,
        vbe_info_block: &'a mut VbeInfoBlock,
    ) -> Result<&'a mut VbeInfoBlock, VesaGetControllerInfoError> {
        type F = unsafe extern "C" fn(u16) -> u16;
        let f = unsafe { mem::transmute::<_, F>(self.0 as usize) };
        let ax = unsafe { f(ptr::from_mut(vbe_info_block).addr().try_into().unwrap()) };
        if ax != 0x004f {
            return Err(VesaGetControllerInfoError::InvalidAx(ax));
        }
        if &vbe_info_block.vbe_signature != b"VESA" {
            return Err(VesaGetControllerInfoError::InvalidSignature(
                vbe_info_block.vbe_signature,
            ));
        }
        Ok(vbe_info_block)
    }
}

pub struct VideoModesIterator {
    pointer: *mut u16,
}

impl VideoModesIterator {
    pub unsafe fn new(vbe_info_block: &VbeInfoBlock) -> Self {
        Self {
            pointer: usize::try_from(
                u32::from(vbe_info_block.video_modes_segment) * 16
                    + u32::from(vbe_info_block.video_modes_offset),
            )
            .unwrap() as *mut u16,
        }
    }
}

impl Iterator for VideoModesIterator {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        let video_mode = unsafe { self.pointer.read_unaligned() };
        if video_mode == 0xffff {
            return None;
        }
        self.pointer = (self.pointer.addr() + size_of::<u16>()) as *mut u16;
        Some(video_mode)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes)]
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

pub struct VesaGetModeInfoPtr(u16);

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

#[derive(Debug, Clone, Copy)]
pub enum VesaGetModeInfoErr {
    InvalidAx(u16),
}

impl VesaGetModeInfoPtr {
    pub fn call<'a>(
        &self,
        mode: u16,
        buffer: &'a mut [u8; size_of::<ModeInfo>()],
    ) -> Result<&'a mut ModeInfo, VesaGetModeInfoErr> {
        type F = unsafe extern "C" fn(u16, u8, u8, u16) -> u16;
        let f = unsafe { mem::transmute::<_, F>(self.0 as usize) };
        let ax = unsafe {
            f(
                ptr::from_mut(buffer).addr().try_into().unwrap(),
                Default::default(),
                Default::default(),
                mode,
            )
        };
        if ax != 0x004f {
            return Err(VesaGetModeInfoErr::InvalidAx(ax));
        }
        Ok(transmute_mut!(buffer))
    }
}

pub struct VesaSetModePtr(u16);

#[derive(Debug, Clone, Copy)]
pub enum VesaSetModeErr {
    InvalidAx(u16),
}

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

impl VesaSetModePtr {
    pub fn call(&self, mode: u9, clear: bool) -> Result<(), VesaSetModeErr> {
        // di -> di
        // si -> es
        // dx -> bx
        type F = unsafe extern "C" fn(u16, u16, u16) -> u16;
        // let ptr = ptr::from_ref(info).addr();
        // let es = u16::try_from(ptr / 16).unwrap_or(u16::MAX);
        // let di = u16::try_from(ptr - usize::try_from(es).unwrap() * 16).unwrap();
        // assert!(mode > 0x1FF);

        let bx = SetVbeModeBx::ZERO
            .with_mode_number(mode)
            .with_refresh_rate(false)
            .with_linear(true)
            .with_dont_clear(!clear)
            .raw_value();
        let f = unsafe { mem::transmute::<_, F>(self.0 as usize) };
        let ax = unsafe { f(0, 0, bx) };
        if ax != 0x004f {
            return Err(VesaSetModeErr::InvalidAx(ax));
        }
        Ok(())
    }
}
