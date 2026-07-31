use core::{borrow::Borrow, fmt::Write};

use log::{Log, max_level, set_logger, set_max_level};
use spin::Once;

use crate::{bios::Int10Ptr, writer_with_cr::WriterWithCr};

struct Logger {
    int_10: Int10Ptr,
}

static LOGGER: Once<Logger> = Once::new();

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= max_level()
    }

    fn log(&self, record: &log::Record) {
        struct Int10Writer<T>(T);
        impl<T: Borrow<Int10Ptr>> Write for Int10Writer<T> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for &b in s.as_bytes() {
                    (self.0.borrow().as_fn())(b)
                }
                Ok(())
            }
        }
        let mut writer = WriterWithCr::new(Int10Writer(self.int_10));
        let msg = record.args();
        writeln!(writer, "{msg}").unwrap();
    }

    fn flush(&self) {}
}

pub fn init(int_10: Int10Ptr) {
    set_max_level(log::LevelFilter::Trace);
    set_logger(LOGGER.call_once(|| Logger { int_10 })).unwrap();
}
