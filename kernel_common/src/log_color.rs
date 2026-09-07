use core::fmt::Display;

use embedded_graphics::{pixelcolor::Rgb888, prelude::RgbColor};
use owo_colors::{AnsiColors, FgDynColorDisplay};

/// Represents a color in a terminal or screen. The default color may depend on if the theme is light or dark.
#[derive(Debug, Clone, Copy)]
pub enum LogColor {
    Default,
    BrightRed,
    BrightYellow,
    BrightBlue,
    BrightCyan,
    BrightMagenta,
}

impl From<log::Level> for LogColor {
    fn from(value: log::Level) -> Self {
        match value {
            log::Level::Error => Self::BrightRed,
            log::Level::Warn => Self::BrightYellow,
            log::Level::Info => Self::BrightBlue,
            log::Level::Debug => Self::BrightCyan,
            log::Level::Trace => Self::BrightMagenta,
        }
    }
}

impl From<LogColor> for AnsiColors {
    fn from(value: LogColor) -> Self {
        match value {
            LogColor::Default => AnsiColors::Default,
            LogColor::BrightRed => AnsiColors::BrightRed,
            LogColor::BrightYellow => AnsiColors::BrightYellow,
            LogColor::BrightBlue => AnsiColors::BrightBlue,
            LogColor::BrightCyan => AnsiColors::BrightCyan,
            LogColor::BrightMagenta => AnsiColors::BrightMagenta,
        }
    }
}

impl From<LogColor> for Rgb888 {
    fn from(value: LogColor) -> Self {
        match value {
            LogColor::Default => Rgb888::WHITE,
            // Mimick the ANSI escape colors
            LogColor::BrightRed => Rgb888::new(255, 85, 85),
            LogColor::BrightYellow => Rgb888::new(255, 255, 85),
            LogColor::BrightBlue => Rgb888::new(85, 85, 255),
            LogColor::BrightCyan => Rgb888::new(85, 255, 255),
            LogColor::BrightMagenta => Rgb888::new(255, 85, 255),
        }
    }
}

impl LogColor {
    pub fn color_msg<'a, T: Display + ?Sized>(
        &self,
        msg: &'a T,
    ) -> FgDynColorDisplay<'a, AnsiColors, T> {
        FgDynColorDisplay::new(msg, (*self).into())
    }
}

impl From<LogColor> for uefi::proto::console::text::Color {
    fn from(value: LogColor) -> Self {
        match value {
            LogColor::Default => Self::White,
            LogColor::BrightRed => Self::LightRed,
            LogColor::BrightYellow => Self::Yellow,
            LogColor::BrightBlue => Self::LightBlue,
            LogColor::BrightCyan => Self::LightCyan,
            LogColor::BrightMagenta => Self::LightMagenta,
        }
    }
}
