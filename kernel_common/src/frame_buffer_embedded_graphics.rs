use core::{convert::Infallible, ops::Range, ptr::NonNull};

use alloc::{boxed::Box, vec};
use embedded_graphics::{
    Pixel,
    pixelcolor::Rgb888,
    prelude::{Dimensions, DrawTarget, Point, Size},
    primitives::Rectangle,
};

use crate::frame_buffer_info::FrameBufferInfo;

enum BufferingMode {
    /// Draw directly to the GPU memory.
    Direct,
    /// Draw to CPU memory and then copy the entire thing to the GPU mem.
    DoubleBuffer {
        cpu_buffer: Box<[u32]>,
        changed_lines: Option<Range<u32>>,
    },
}

pub struct FrameBufferEmbeddedGraphics<'a> {
    gpu_buffer: &'a mut [u32],
    buffering_mode: BufferingMode,
    info: FrameBufferInfo,
    pixel_pitch: usize,
    bounding_box: Rectangle,
}

impl FrameBufferEmbeddedGraphics<'_> {
    /// # Safety
    /// The frame buffer must be mapped at `addr`
    pub unsafe fn new(addr: NonNull<u32>, info: FrameBufferInfo, double_buffer: bool) -> Self {
        if info.bits_per_pixel as u32 == u32::BITS {
            let buffer_len =
                (info.bytes_per_horizontal_line * info.height) as usize / size_of::<u32>();
            Self {
                gpu_buffer: {
                    let mut ptr = NonNull::slice_from_raw_parts(addr, buffer_len);
                    // Safety: This memory is mapped
                    unsafe { ptr.as_mut() }
                },
                buffering_mode: if double_buffer {
                    BufferingMode::DoubleBuffer {
                        cpu_buffer: vec![0; buffer_len].into_boxed_slice(),
                        changed_lines: None,
                    }
                } else {
                    BufferingMode::Direct
                },
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

    /// Get the CPU buffer if double buffering is enabled, otherwise get the GPU buffer.
    fn buffer_mut(&mut self) -> &mut [u32] {
        match &mut self.buffering_mode {
            BufferingMode::Direct => self.gpu_buffer,
            BufferingMode::DoubleBuffer { cpu_buffer, .. } => cpu_buffer,
        }
    }

    pub fn flush(&mut self) {
        if let BufferingMode::DoubleBuffer {
            cpu_buffer,
            changed_lines,
        } = &mut self.buffering_mode
            && let Some(changed_lines) = changed_lines.take()
        {
            let elems_to_copy = usize::try_from(changed_lines.start).unwrap()
                * usize::try_from(self.info.bytes_per_horizontal_line / 4).unwrap()
                ..usize::try_from(changed_lines.end).unwrap()
                    * usize::try_from(self.info.bytes_per_horizontal_line / 4).unwrap();
            self.gpu_buffer[elems_to_copy.clone()].copy_from_slice(&cpu_buffer[elems_to_copy]);
        }
    }

    /// Moves everything on the screen up, leaving the bottom the same as it was before
    pub fn shift_up(&mut self, amount: usize) {
        let src = amount * self.pixel_pitch..;
        self.buffer_mut().copy_within(src, 0);
        if let BufferingMode::DoubleBuffer { changed_lines, .. } = &mut self.buffering_mode {
            *changed_lines = Some(0..u32::try_from(self.info.height).unwrap());
        }
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
                if let BufferingMode::DoubleBuffer { changed_lines, .. } = &mut self.buffering_mode
                {
                    let point_y = u32::try_from(point.y).unwrap();
                    if let Some(changed_lines) = changed_lines {
                        if point_y < changed_lines.start {
                            changed_lines.start = point_y;
                        }
                        if point_y >= changed_lines.end {
                            changed_lines.end = point_y + 1;
                        }
                    } else {
                        *changed_lines = Some(point_y..point_y + 1)
                    }
                }
                let pixel_index = point.y as usize * self.pixel_pitch + point.x as usize;
                self.buffer_mut()[pixel_index] = self.info.pixel_info.build_pixel(color);
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
            let pixels = &mut self.buffer_mut()[pixel_index..pixel_index + width];
            pixels.fill(pixel);
        }
        if let BufferingMode::DoubleBuffer { changed_lines, .. } = &mut self.buffering_mode {
            let start_y = u32::try_from(area.top_left.y).unwrap();
            let end_y = u32::try_from(area.top_left.y).unwrap() + area.size.height;
            if let Some(changed_lines) = changed_lines {
                if start_y < changed_lines.start {
                    changed_lines.start = start_y;
                }
                if end_y > changed_lines.end {
                    changed_lines.end = end_y;
                }
            } else {
                *changed_lines = Some(start_y..end_y);
            }
        }
        Ok(())
    }
}
