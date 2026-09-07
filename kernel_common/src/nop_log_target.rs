use crate::log_target::LogTarget;

pub struct NopLogTarget;

impl LogTarget for NopLogTarget {
    fn write_with_color(
        &mut self,
        _color: crate::log_color::LogColor,
        _msg: &dyn core::fmt::Display,
    ) {
    }

    fn flush(&mut self) {}
}
