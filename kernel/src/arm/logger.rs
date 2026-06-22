use core::{fmt::Write, ptr::NonNull};

use aarch32_cpu::register::Cpsr;
use arm_pl011_uart::{Uart, UniqueMmioPointer};
use log::max_level;
use spin::{Mutex, Once};

struct Logger {
    uart: Mutex<Uart<'static>>,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= max_level()
    }

    fn log(&self, record: &log::Record) {
        let prev_cpsr = Cpsr::read();
        unsafe { Cpsr::write(prev_cpsr.with_i(false)) };
        let mut console = self.uart.lock();
        writeln!(console, "{}", record.args()).unwrap();
        drop(console);
        unsafe { Cpsr::write(prev_cpsr) };
    }

    fn flush(&self) {}
}
static LOGGER: Once<Logger> = Once::new();

/// # Safety
/// Only call this function once
pub unsafe fn init() {
    let logger = LOGGER.call_once(|| Logger {
        uart: Mutex::new(unsafe {
            // FIXME: This is just for testing and is hard-coded for a Raspberry Pi 3B. Use the device tree!
            Uart::new(UniqueMmioPointer::new(
                NonNull::new(0x2020_1000 as *mut _).unwrap(),
            ))
        }),
    });
    // Safety: nothing else is calling this function
    unsafe { log::set_logger_racy(logger).unwrap() };
    // Safety: nothing else is calling this function
    unsafe { log::set_max_level_racy(log::LevelFilter::Trace) };
}

pub unsafe fn force_unlock() {
    unsafe { LOGGER.get().unwrap().uart.force_unlock() };
}

pub fn uart() -> &'static Mutex<Uart<'static>> {
    &LOGGER.get().unwrap().uart
}
