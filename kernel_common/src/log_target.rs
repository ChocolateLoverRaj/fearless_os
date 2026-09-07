use core::fmt::Display;

use crate::log_color::LogColor;

pub trait LogTarget {
    fn write_with_color(&mut self, color: LogColor, msg: &dyn Display);
    fn flush(&mut self);
}
