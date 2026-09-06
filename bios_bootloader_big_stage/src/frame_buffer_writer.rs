use core::fmt::Write;

use embedded_graphics::Drawable;
use embedded_graphics::mono_font::{MonoFont, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{Dimensions, DrawTarget, Point, Primitive, RgbColor, Size};
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use unicode_segmentation::UnicodeSegmentation;

use crate::frame_buffer_embedded_graphics::FrameBufferEmbeddedGraphics;

pub struct FrameBufferWriter<'a> {
    pub display: &'a mut FrameBufferEmbeddedGraphics<'static>,
    pub position: &'a mut Point,
    pub text_color: <FrameBufferEmbeddedGraphics<'a> as DrawTarget>::Color,
    pub font: MonoFont<'a>,
}

impl Write for FrameBufferWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let background_color = Rgb888::BLACK;
        for c in s.graphemes(true) {
            let height_not_seen = self.position.y + self.font.character_size.height as i32
                - self.display.bounding_box().size.height as i32;
            if height_not_seen > 0 {
                self.display.shift_up(height_not_seen as usize);
                self.position.y -= height_not_seen;
            }
            match c {
                "\r" => {
                    // We do not handle special cursor movements
                }
                "\n" | "\r\n" => {
                    // Fill the remaining space with background color
                    Rectangle::new(
                        *self.position,
                        Size::new(
                            self.display.bounding_box().size.width - self.position.x as u32,
                            self.font.character_size.height,
                        ),
                    )
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .fill_color(background_color)
                            .build(),
                    )
                    .draw(self.display)
                    .map_err(|_| core::fmt::Error)?;
                    self.position.y += self.font.character_size.height as i32;
                    self.position.x = 0;
                }
                c => {
                    let style = MonoTextStyleBuilder::new()
                        .font(&self.font)
                        .text_color(self.text_color)
                        .background_color(background_color)
                        .build();
                    *self.position = Text::with_baseline(c, *self.position, style, Baseline::Top)
                        .draw(self.display)
                        .map_err(|_| core::fmt::Error)?;
                    if self.position.x as u32 + self.font.character_size.width
                        > self.display.bounding_box().size.width
                    {
                        self.position.y += self.font.character_size.height as i32;
                        self.position.x = 0;
                    }
                }
            }
        }
        Ok(())
    }
}
