use core::{fmt::Write, ptr::NonNull};

use aarch64_cpu::registers::{DAIF, ReadWriteable, Readable, Writeable};
use log::max_level;
use spin::{Mutex, Once};

use super::bcm2835_aux_uart::Bcm2835AuxUart;

struct Logger {
    uart: Mutex<Bcm2835AuxUart>,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= max_level()
    }

    fn log(&self, record: &log::Record) {
        let prev_daif = DAIF.get();
        DAIF.modify(DAIF::D::SET + DAIF::A::SET + DAIF::I::SET + DAIF::F::SET);
        let mut console = self.uart.lock();
        writeln!(console, "{}", record.args()).unwrap();
        drop(console);
        DAIF.set(prev_daif);
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
            Bcm2835AuxUart::new(NonNull::new(0x3F215040 as *mut _).unwrap())
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

pub fn uart() -> &'static Mutex<Bcm2835AuxUart> {
    &LOGGER.get().unwrap().uart
}
