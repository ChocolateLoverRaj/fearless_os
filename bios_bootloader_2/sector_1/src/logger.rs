use core::fmt::Write;

use log::{Log, max_level, set_logger, set_max_level};

use crate::{bios::int_10, writer_with_cr::WriterWithCr};

struct Logger;

static LOGGER: Logger = Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= max_level()
    }

    fn log(&self, record: &log::Record) {
        struct Int10Writer;
        impl Write for Int10Writer {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for &b in s.as_bytes() {
                    int_10(b);
                }
                Ok(())
            }
        }
        let mut writer = WriterWithCr::new(Int10Writer);
        let msg = record.args();
        writeln!(writer, "{msg}").unwrap();
    }

    fn flush(&self) {}
}

pub fn init() {
    set_max_level(log::LevelFilter::Info);
    set_logger(&LOGGER).unwrap();
}
