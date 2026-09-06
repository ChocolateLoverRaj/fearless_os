use core::fmt::Write;

use uart_16550::{Uart16550Tty, backend::PioBackend};

use crate::log_target::LogTarget;

pub struct UartLogTarget {
    uart: Uart16550Tty<PioBackend>,
}

impl UartLogTarget {
    pub fn new(uart: Uart16550Tty<PioBackend>) -> Self {
        Self { uart }
    }
}

impl LogTarget for UartLogTarget {
    fn write_with_color(&mut self, color: crate::log_target::Color, msg: &dyn core::fmt::Display) {
        let msg = color.color_msg(msg);
        write!(&mut self.uart, "{msg}");
    }

    fn flush(&mut self) {}
}
