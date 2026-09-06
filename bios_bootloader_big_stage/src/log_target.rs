use core::fmt::Display;

use embedded_graphics::{pixelcolor::Rgb888, prelude::RgbColor};
use owo_colors::{AnsiColors, FgDynColorDisplay};

/// Represents a color in a terminal or screen. The default color may depend on if the theme is light or dark.
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Default,
    BrightRed,
    BrightYellow,
    BrightBlue,
    BrightCyan,
    BrightMagenta,
}

impl From<log::Level> for Color {
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

impl From<Color> for AnsiColors {
    fn from(value: Color) -> Self {
        match value {
            Color::Default => AnsiColors::Default,
            Color::BrightRed => AnsiColors::BrightRed,
            Color::BrightYellow => AnsiColors::BrightYellow,
            Color::BrightBlue => AnsiColors::BrightBlue,
            Color::BrightCyan => AnsiColors::BrightCyan,
            Color::BrightMagenta => AnsiColors::BrightMagenta,
        }
    }
}

impl From<Color> for Rgb888 {
    fn from(value: Color) -> Self {
        match value {
            Color::Default => Rgb888::WHITE,
            // Mimick the ANSI escape colors
            Color::BrightRed => Rgb888::new(255, 85, 85),
            Color::BrightYellow => Rgb888::new(255, 255, 85),
            Color::BrightBlue => Rgb888::new(85, 85, 255),
            Color::BrightCyan => Rgb888::new(85, 255, 255),
            Color::BrightMagenta => Rgb888::new(255, 85, 255),
        }
    }
}

impl Color {
    pub fn color_msg<'a, T: Display + ?Sized>(
        &self,
        msg: &'a T,
    ) -> FgDynColorDisplay<'a, AnsiColors, T> {
        FgDynColorDisplay::new(msg, (*self).into())
    }
}

pub trait LogTarget {
    fn write_with_color(&mut self, color: Color, msg: &dyn Display);
    fn flush(&mut self);
}
