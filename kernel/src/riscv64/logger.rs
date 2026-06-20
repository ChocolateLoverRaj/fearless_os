use core::fmt::Write;

use log::max_level;
use riscv::{interrupt, register::sstatus};
use spin::{Mutex, Once};

use super::console::Console;

struct Logger {
    console: Mutex<Console>,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= max_level()
    }

    fn log(&self, record: &log::Record) {
        let interrupts_were_enabled = sstatus::read().sie();
        if interrupts_were_enabled {
            interrupt::disable();
        }
        let mut console = self.console.lock();
        writeln!(console, "{}", record.args()).unwrap();
        drop(console);
        if interrupts_were_enabled {
            unsafe { interrupt::enable() };
        }
    }

    fn flush(&self) {}
}
static LOGGER: Once<Logger> = Once::new();

/// # Safety
/// Only call this function once
pub unsafe fn init() {
    let logger = LOGGER.call_once(|| Logger {
        console: Mutex::new(Console::take().unwrap()),
    });
    // Safety: nothing else is calling this function
    unsafe { log::set_logger_racy(logger).unwrap() };
    // Safety: nothing else is calling this function
    unsafe { log::set_max_level_racy(log::LevelFilter::Trace) };
}

pub unsafe fn force_unlock() {
    unsafe { LOGGER.get().unwrap().console.force_unlock() };
}
