use core::fmt::Write;

use log::{Log, max_level, set_logger, set_max_level};
use spin::Once;

use crate::{bios::BiosFns, writer_with_cr::WriterWithCr};

struct Logger {
    bios_fns: BiosFns,
}

static LOGGER: Once<Logger> = Once::new();

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= max_level()
    }

    fn log(&self, record: &log::Record) {
        struct Int10Writer {
            bios_fns: BiosFns,
        }
        impl Write for Int10Writer {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for &b in s.as_bytes() {
                    self.bios_fns.print_char(b);
                }
                Ok(())
            }
        }
        let mut writer = WriterWithCr::new(Int10Writer {
            bios_fns: self.bios_fns,
        });
        let msg = record.args();
        writeln!(writer, "{msg}").unwrap();
    }

    fn flush(&self) {}
}

pub fn init(bios_functions: BiosFns) {
    set_max_level(log::LevelFilter::Info);
    set_logger(LOGGER.call_once(|| Logger {
        bios_fns: bios_functions,
    }))
    .unwrap();
}
