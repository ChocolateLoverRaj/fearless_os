use core::{convert::Infallible, ptr::NonNull};

use alloc::{boxed::Box, vec};
use embedded_graphics::{
    Pixel,
    pixelcolor::Rgb888,
    prelude::{Dimensions, DrawTarget, Point, Size},
    primitives::Rectangle,
};

use crate::frame_buffer_info::FrameBufferInfo;

pub struct FrameBufferEmbeddedGraphics<'a> {
    gpu_buffer: &'a mut [u32],
    our_buffer: Box<[u32]>,
    info: FrameBufferInfo,
    pixel_pitch: usize,
    bounding_box: Rectangle,
}

impl FrameBufferEmbeddedGraphics<'_> {
    /// # Safety
    /// The frame buffer must be mapped at `addr`
    pub unsafe fn new(addr: NonNull<u32>, info: FrameBufferInfo) -> Self {
        if info.bits_per_pixel as u32 == u32::BITS {
            let buffer_len =
                (info.bytes_per_horizontal_line * info.height) as usize / size_of::<u32>();
            Self {
                gpu_buffer: {
                    let mut ptr = NonNull::slice_from_raw_parts(addr, buffer_len);
                    // Safety: This memory is mapped
                    unsafe { ptr.as_mut() }
                },
                our_buffer: vec![0; buffer_len].into_boxed_slice(),
                info,
                pixel_pitch: info.bytes_per_horizontal_line as usize / size_of::<u32>(),
                bounding_box: Rectangle {
                    top_left: Point::zero(),
                    size: Size {
                        width: info.width.try_into().unwrap(),
                        height: info.height.try_into().unwrap(),
                    },
                },
            }
        } else {
            panic!("DrawTarget implemented for RGB888, but bpp doesn't match RGB888");
        }
    }
}

impl FrameBufferEmbeddedGraphics<'_> {
    pub fn flush(&mut self) {
        self.gpu_buffer.copy_from_slice(&self.our_buffer);
    }

    /// Moves everything on the screen up, leaving the bottom the same as it was before
    pub fn shift_up(&mut self, amount: usize) {
        self.our_buffer.copy_within(amount * self.pixel_pitch.., 0);
    }
}

impl Dimensions for FrameBufferEmbeddedGraphics<'_> {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        self.bounding_box
    }
}

impl DrawTarget for FrameBufferEmbeddedGraphics<'_> {
    type Color = Rgb888;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        let bounding_box = self.bounding_box();
        pixels
            .into_iter()
            .filter(|Pixel(point, _)| bounding_box.contains(*point))
            .for_each(|Pixel(point, color)| {
                let pixel_index = point.y as usize * self.pixel_pitch + point.x as usize;
                self.our_buffer[pixel_index] = self.info.pixel_info.build_pixel(color);
            });
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let area = area.intersection(&self.bounding_box);
        let pixel = self.info.pixel_info.build_pixel(color);
        let width = area.size.width as usize;
        let top_left_x = area.top_left.x as usize;
        for y in area.top_left.y as usize..area.top_left.y as usize + area.size.height as usize {
            let pixel_index = y * self.pixel_pitch + top_left_x;
            let pixels = &mut self.our_buffer[pixel_index..pixel_index + width];
            pixels.fill(pixel);
        }
        Ok(())
    }
}
