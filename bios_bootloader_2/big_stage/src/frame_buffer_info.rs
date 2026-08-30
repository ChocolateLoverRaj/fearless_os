use common::bios::vesa::ModeInfo;

use crate::rgb_pixel_info::RgbPixelInfo;

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct FrameBufferInfo {
    pub width: u64,
    pub height: u64,
    pub bytes_per_horizontal_line: u64,
    pub bits_per_pixel: u16,
    pub pixel_info: RgbPixelInfo,
}

impl From<&ModeInfo> for FrameBufferInfo {
    fn from(value: &ModeInfo) -> Self {
        Self {
            width: value.x_resolution.get().into(),
            height: value.y_resolution.get().into(),
            bytes_per_horizontal_line: value.lin_bytes_per_scan.get().into(),
            bits_per_pixel: value.bits_per_pixel.into(),
            pixel_info: RgbPixelInfo {
                red_mask_shift: value.lin_red_field_position,
                red_mask_size: value.lin_red_mask_size,
                green_mask_shift: value.lin_green_field_position,
                green_mask_size: value.lin_green_mask_size,
                blue_mask_shift: value.lin_blue_field_position,
                blue_mask_size: value.lin_blue_mask_size,
            },
        }
    }
}

// impl From<&limine::framebuffer::Framebuffer<'_>> for FrameBufferInfo {
//     fn from(framebuffer: &limine::framebuffer::Framebuffer) -> Self {
//         FrameBufferInfo {
//             width: framebuffer.width(),
//             height: framebuffer.height(),
//             pitch: framebuffer.pitch(),
//             bits_per_pixel: framebuffer.bpp(),
//             pixel_info: RgbPixelInfo {
//                 red_mask_size: framebuffer.red_mask_size(),
//                 red_mask_shift: framebuffer.red_mask_shift(),
//                 green_mask_size: framebuffer.green_mask_size(),
//                 green_mask_shift: framebuffer.green_mask_shift(),
//                 blue_mask_size: framebuffer.blue_mask_size(),
//                 blue_mask_shift: framebuffer.blue_mask_shift(),
//             },
//         }
//     }
// }
