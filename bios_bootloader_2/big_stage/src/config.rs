use embedded_graphics::mono_font::{self, MonoFont};
use log::LevelFilter;

pub struct ScreenConfig {
    pub width: u16,
    pub height: u16,
    pub bpp: u8,
}

pub enum ScreenFlush {
    EveryLog,
    Manually,
}

pub struct Config {
    pub preffered_resolution: Option<ScreenConfig>,
    pub screen_log_level: LevelFilter,
    pub serial_log_level: LevelFilter,
    /// If true, will log to the screen and not UART, even if a UART is supported.
    pub prefer_screen_logging: bool,
    pub font: MonoFont<'static>,
    pub screen_flush: ScreenFlush,
}

pub const CONFIG: Config = Config {
    preffered_resolution: Some(ScreenConfig {
        width: 1024,
        height: 768,
        bpp: 32,
    }),
    screen_log_level: LevelFilter::Info,
    serial_log_level: LevelFilter::Debug,
    prefer_screen_logging: true,
    font: mono_font::iso_8859_16::FONT_6X13,
    screen_flush: ScreenFlush::Manually,
};
