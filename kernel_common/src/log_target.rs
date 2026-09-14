use core::fmt::Display;

use crate::log_color::LogColor;

pub trait LogTarget {
    fn write_with_color(&mut self, color: LogColor, msg: &dyn Display);
    fn flush(&mut self);
}

impl<T: ?Sized + LogTarget> LogTarget for &mut T {
    fn write_with_color(&mut self, color: LogColor, msg: &dyn Display) {
        T::write_with_color(self, color, msg);
    }

    fn flush(&mut self) {
        T::flush(self);
    }
}
