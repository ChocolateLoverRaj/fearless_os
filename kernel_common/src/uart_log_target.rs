use core::{borrow::BorrowMut, fmt::Write, marker::PhantomData};

use crate::{log_color::LogColor, log_target::LogTarget};

pub struct UartLogTarget<T, U: ?Sized = T> {
    uart: T,
    target: PhantomData<U>,
}

impl<T, U: ?Sized> UartLogTarget<T, U> {
    pub fn new(uart: T) -> Self {
        Self {
            uart,
            target: PhantomData,
        }
    }
}

impl<T, U: ?Sized> LogTarget for UartLogTarget<T, U>
where
    T: BorrowMut<U>,
    U: Write,
{
    fn write_with_color(&mut self, color: LogColor, msg: &dyn core::fmt::Display) {
        let msg = color.color_msg(msg);
        let _ = write!(self.uart.borrow_mut(), "{msg}");
    }

    fn flush(&mut self) {}
}
