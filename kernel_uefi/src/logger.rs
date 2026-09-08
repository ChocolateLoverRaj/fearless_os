use kernel_common::{
    config::CONFIG,
    frame_buffer_embedded_graphics::FrameBufferEmbeddedGraphics,
    frame_buffer_log_target::FrameBufferLogTarget,
    log_color::LogColor,
    log_target::LogTarget,
    logger::{Logger, LoggerInner},
};
use log::LevelFilter;
use spin::Once;
use uefi::{
    boot::{get_handle_for_protocol, open_protocol_exclusive},
    proto::console::text::{Color, Output},
};

use core::fmt::Write;

struct UefiTextOutput;

impl LogTarget for UefiTextOutput {
    fn write_with_color(&mut self, color: LogColor, msg: &dyn core::fmt::Display) {
        if let Ok(handle) = get_handle_for_protocol::<Output>() {
            if let Ok(mut output) = open_protocol_exclusive::<Output>(handle) {
                output.set_color(color.into(), Color::Black).unwrap();
                let _ = write!(output, "{msg}");
                // panic!("");
            } else {
                // panic!("");
            }
        } else {
            // panic!("");
        }
    }

    fn flush(&mut self) {}
}

enum UefiLoggerInner {
    UefiTextOutput(UefiTextOutput),
    UefiGop(FrameBufferLogTarget),
}

impl LoggerInner for UefiLoggerInner {
    fn target_mut(&mut self) -> &mut dyn LogTarget {
        match self {
            Self::UefiTextOutput(v) => v,
            Self::UefiGop(v) => v,
        }
    }

    fn level_filter(&self) -> log::LevelFilter {
        CONFIG.screen_log_level
    }
}

static LOGGER: Once<Logger<UefiLoggerInner>> = Once::new();

pub fn init() {
    if let Ok(handle) = get_handle_for_protocol::<Output>() {
        if let Ok(mut a) = open_protocol_exclusive::<Output>(handle) {
            let _ = a.enable_cursor(false);
        }
    }
    log::set_logger(
        LOGGER.call_once(|| Logger::new(UefiLoggerInner::UefiTextOutput(UefiTextOutput))),
    );
    log::set_max_level(LevelFilter::Trace);
}

pub fn switch_to_gop(frame_buffer: FrameBufferEmbeddedGraphics<'static>) {
    LOGGER.get().unwrap().update(|logger| {
        *logger = UefiLoggerInner::UefiGop(FrameBufferLogTarget::new(frame_buffer));
    })
}
