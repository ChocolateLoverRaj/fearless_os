use core::cmp::max;

use bios_bootloader_common::bios::BiosFns;
use kernel_common::{
    frame_buffer_embedded_graphics::FrameBufferEmbeddedGraphics,
    frame_buffer_log_target::FrameBufferLogTarget,
    log_target::LogTarget,
    logger::{Logger, LoggerInner},
};
use log::{LevelFilter, set_logger, set_max_level};
use spin::Once;
use uart_16550::{Uart16550Tty, backend::PioBackend};

use crate::{
    config::CONFIG, uart_log_target::UartLogTarget, vesa_text_log_target::VesaTextLogTarget,
};

enum LoggerData {
    VesaText(VesaTextLogTarget),
    Uart(UartLogTarget),
    FrameBuffer(FrameBufferLogTarget),
}

impl LoggerInner for LoggerData {
    fn target_mut(&mut self) -> &mut dyn LogTarget {
        match self {
            LoggerData::VesaText(target) => target,
            LoggerData::Uart(target) => target,
            LoggerData::FrameBuffer(target) => target,
        }
    }

    fn level_filter(&self) -> LevelFilter {
        match self {
            LoggerData::VesaText(_) | LoggerData::FrameBuffer(_) => CONFIG.screen_log_level,
            LoggerData::Uart(_) => CONFIG.serial_log_level,
        }
    }
}

static LOGGER: Once<Logger<LoggerData>> = Once::new();

pub fn init(bios_fns: BiosFns) {
    set_max_level(max(CONFIG.screen_log_level, CONFIG.serial_log_level));
    set_logger(
        LOGGER.call_once(|| Logger::new(LoggerData::VesaText(VesaTextLogTarget::new(bios_fns)))),
    );
}

pub fn init_uart(uart: Uart16550Tty<PioBackend>) {
    LOGGER.get().unwrap().update(|logger| {
        *logger = LoggerData::Uart(UartLogTarget::new(uart));
    });
}

/// Logs to the frame buffer instead of VBE text mode logging.
/// If specified, will do this even if logging to a UART was specified.
pub fn init_frame_buffer(
    frame_buffer: FrameBufferEmbeddedGraphics<'static>,
    replace_uart: bool,
) -> Option<FrameBufferEmbeddedGraphics<'static>> {
    LOGGER.get().unwrap().update(|logger| {
        if !matches!(logger, LoggerData::Uart(_)) || replace_uart {
            *logger = LoggerData::FrameBuffer(FrameBufferLogTarget::new(frame_buffer));
            None
        } else {
            Some(frame_buffer)
        }
    })
}
