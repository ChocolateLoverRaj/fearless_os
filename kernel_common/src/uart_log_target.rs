use core::fmt::Write;

use crate::{log_color::LogColor, log_target::LogTarget};
use uart_16550::{Uart16550Tty, backend::PioBackend};

pub struct UartLogTarget {
    uart: Uart16550Tty<PioBackend>,
}

impl UartLogTarget {
    pub fn new(uart: Uart16550Tty<PioBackend>) -> Self {
        Self { uart }
    }
}

impl LogTarget for UartLogTarget {
    fn write_with_color(&mut self, color: LogColor, msg: &dyn core::fmt::Display) {
        let msg = color.color_msg(msg);
        let _ = write!(&mut self.uart, "{msg}");
    }

    fn flush(&mut self) {}
}
