use kernel_common::{
    log_color::LogColor,
    log_target::LogTarget,
    logger::{Logger, LoggerInner},
};
use log::{LevelFilter, max_level};
use spin::Once;
use uefi::{
    boot::{get_handle_for_protocol, open_protocol_exclusive},
    proto::console::text::{Color, Output},
};

use core::fmt::Write;

struct UefiLoggerInner;

impl LogTarget for UefiLoggerInner {
    fn write_with_color(&mut self, color: LogColor, msg: &dyn core::fmt::Display) {
        if let Ok(handle) = get_handle_for_protocol::<Output>() {
            if let Ok(mut output) = open_protocol_exclusive::<Output>(handle) {
                output.set_color(color.into(), Color::Black).unwrap();
                write!(output, "{msg}").unwrap();
            }
        }
    }

    fn flush(&mut self) {}
}

impl LoggerInner for UefiLoggerInner {
    fn target_mut(&mut self) -> &mut dyn LogTarget {
        self
    }

    fn level_filter(&self) -> log::LevelFilter {
        max_level()
    }
}

static LOGGER: Once<Logger<UefiLoggerInner>> = Once::new();

pub fn init() {
    if let Ok(handle) = get_handle_for_protocol::<Output>() {
        if let Ok(mut a) = open_protocol_exclusive::<Output>(handle) {
            let _ = a.enable_cursor(false);
        }
    }
    log::set_logger(LOGGER.call_once(|| Logger::new(UefiLoggerInner)));
    log::set_max_level(LevelFilter::Trace);
}
