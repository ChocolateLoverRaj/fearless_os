// use bios_bootloader_common::bios::vesa::ModeInfo;

use uefi::proto::console::gop::PixelFormat;

use crate::rgb_pixel_info::RgbPixelInfo;

#[derive(Debug, Clone, Copy)]
pub struct FrameBufferInfo {
    pub width: u64,
    pub height: u64,
    pub bytes_per_horizontal_line: u64,
    pub bits_per_pixel: u16,
    pub pixel_info: RgbPixelInfo,
}

// impl From<&ModeInfo> for FrameBufferInfo {
//     fn from(value: &ModeInfo) -> Self {
//         Self {
//             width: value.x_resolution.get().into(),
//             height: value.y_resolution.get().into(),
//             bytes_per_horizontal_line: value.lin_bytes_per_scan.get().into(),
//             bits_per_pixel: value.bits_per_pixel.into(),
//             pixel_info: RgbPixelInfo {
//                 red_mask_shift: value.lin_red_field_position,
//                 red_mask_size: value.lin_red_mask_size,
//                 green_mask_shift: value.lin_green_field_position,
//                 green_mask_size: value.lin_green_mask_size,
//                 blue_mask_shift: value.lin_blue_field_position,
//                 blue_mask_size: value.lin_blue_mask_size,
//             },
//         }
//     }
// }

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

impl TryFrom<&uefi::proto::console::gop::ModeInfo> for FrameBufferInfo {
    type Error = FromUefiError;

    fn try_from(value: &uefi::proto::console::gop::ModeInfo) -> Result<Self, Self::Error> {
        Ok({
            let (width, height) = value.resolution();
            FrameBufferInfo {
                width: width.try_into().unwrap(),
                height: height.try_into().unwrap(),
                bytes_per_horizontal_line: (value.stride() * 4).try_into().unwrap(),
                bits_per_pixel: 32,
                pixel_info: match value.pixel_format() {
                    PixelFormat::BltOnly => Err(FromUefiError::BltOnly)?,
                    PixelFormat::Rgb => RgbPixelInfo {
                        red_mask_size: 8,
                        red_mask_shift: 0,
                        green_mask_size: 8,
                        green_mask_shift: 8,
                        blue_mask_size: 8,
                        blue_mask_shift: 16,
                    },
                    PixelFormat::Bgr => RgbPixelInfo {
                        red_mask_size: 8,
                        red_mask_shift: 16,
                        green_mask_size: 8,
                        green_mask_shift: 8,
                        blue_mask_size: 8,
                        blue_mask_shift: 0,
                    },
                    PixelFormat::Bitmask => {
                        let b = value.pixel_bitmask().ok_or(FromUefiError::NoBitmask)?;
                        RgbPixelInfo {
                            red_mask_size: b.red.count_ones().try_into().unwrap(),
                            red_mask_shift: b.red.trailing_zeros().try_into().unwrap(),
                            green_mask_size: b.green.count_ones().try_into().unwrap(),
                            green_mask_shift: b.green.trailing_zeros().try_into().unwrap(),
                            blue_mask_size: b.blue.count_ones().try_into().unwrap(),
                            blue_mask_shift: b.blue.trailing_zeros().try_into().unwrap(),
                        }
                    }
                },
            }
        })
    }
}

#[derive(Debug)]
pub enum FromUefiError {
    /// GOP does not support frame buffer.
    BltOnly,
    NoBitmask,
}
