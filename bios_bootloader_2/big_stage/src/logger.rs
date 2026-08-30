use core::cmp::max;

use common::bios::BiosFns;
use log::{LevelFilter, Log, set_logger, set_max_level};
use spin::{Mutex, Once};
use uart_16550::{Uart16550Tty, backend::PioBackend};

use crate::{
    config::{CONFIG, ScreenFlush},
    frame_buffer_embedded_graphics::FrameBufferEmbeddedGraphics,
    frame_buffer_log_target::FrameBufferLogTarget,
    log_target::{Color, LogTarget},
    uart_log_target::UartLogTarget,
    vesa_text_log_target::VesaTextLogTarget,
};

enum LoggerKind {
    VesaText(VesaTextLogTarget),
    Uart(UartLogTarget),
    FrameBuffer(FrameBufferLogTarget),
}

struct LoggerData {
    logger: LoggerKind,
}

impl LoggerData {
    fn target_mut(&mut self) -> &mut dyn LogTarget {
        match &mut self.logger {
            LoggerKind::VesaText(target) => target,
            LoggerKind::Uart(target) => target,
            LoggerKind::FrameBuffer(target) => target,
        }
    }

    fn level_filter(&self) -> LevelFilter {
        match self.logger {
            LoggerKind::VesaText(_) | LoggerKind::FrameBuffer(_) => CONFIG.screen_log_level,
            LoggerKind::Uart(_) => CONFIG.serial_log_level,
        }
    }
}

struct Logger {
    data: Mutex<LoggerData>,
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.data.lock().level_filter()
    }

    fn log(&self, record: &log::Record) {
        let msg = record.args();
        let mut data = self.data.lock();
        let level = record.level();
        if level <= data.level_filter() {
            let target = data.target_mut();
            target.write_with_color(level.into(), &format_args!("{level:5} "));
            target.write_with_color(Color::Default, &format_args!("{msg}\n"));
            if matches!(CONFIG.screen_flush, ScreenFlush::EveryLog) {
                target.flush();
            }
        }
    }

    fn flush(&self) {
        self.data.lock().target_mut().flush();
    }
}

static LOGGER: Once<Logger> = Once::new();

pub fn init(bios_fns: BiosFns) {
    set_max_level(max(CONFIG.screen_log_level, CONFIG.serial_log_level));
    set_logger(LOGGER.call_once(|| Logger {
        data: Mutex::new(LoggerData {
            logger: LoggerKind::VesaText(VesaTextLogTarget::new(bios_fns)),
        }),
    }));
}

pub fn init_uart(uart: Uart16550Tty<PioBackend>) {
    LOGGER.get().unwrap().data.lock().logger = LoggerKind::Uart(UartLogTarget::new(uart));
}

/// Logs to the frame buffer instead of VBE text mode logging.
/// If specified, will do this even if logging to a UART was specified.
pub fn init_frame_buffer(
    frame_buffer: FrameBufferEmbeddedGraphics<'static>,
    replace_uart: bool,
) -> Option<FrameBufferEmbeddedGraphics<'static>> {
    let mut data = LOGGER.get().unwrap().data.lock();
    if !matches!(data.logger, LoggerKind::Uart(_)) || replace_uart {
        data.logger = LoggerKind::FrameBuffer(FrameBufferLogTarget::new(frame_buffer));
        None
    } else {
        Some(frame_buffer)
    }
}
