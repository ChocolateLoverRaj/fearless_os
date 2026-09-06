use core::fmt::Write;

use embedded_graphics::prelude::Point;

use crate::{
    config::CONFIG, frame_buffer_embedded_graphics::FrameBufferEmbeddedGraphics,
    frame_buffer_writer::FrameBufferWriter, log_target::LogTarget,
};

pub struct FrameBufferLogTarget {
    frame_buffer: FrameBufferEmbeddedGraphics<'static>,
    position: Point,
}

impl FrameBufferLogTarget {
    pub fn new(frame_buffer: FrameBufferEmbeddedGraphics<'static>) -> Self {
        Self {
            frame_buffer,
            position: Default::default(),
        }
    }
}

impl LogTarget for FrameBufferLogTarget {
    fn write_with_color(&mut self, color: crate::log_target::Color, msg: &dyn core::fmt::Display) {
        let mut writer = FrameBufferWriter {
            display: &mut self.frame_buffer,
            position: &mut self.position,
            text_color: color.into(),
            font: CONFIG.font,
        };
        write!(writer, "{msg}");
    }

    fn flush(&mut self) {
        self.frame_buffer.flush();
    }
}
