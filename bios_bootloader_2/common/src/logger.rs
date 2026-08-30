use core::{cmp::min, fmt::Write};

use log::{Log, max_level, set_logger, set_max_level};
use spin::Once;

use crate::{bios::BiosFns, writer_with_cr::WriterWithCr};

pub struct VesaConsoleLogger {
    bios_fns: BiosFns,
}

static LOGGER: Once<VesaConsoleLogger> = Once::new();
static DYN_LOGGER: Once<&'static dyn Log> = Once::new();

impl Log for VesaConsoleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= max_level()
    }

    fn log(&self, record: &log::Record) {
        struct Int10Writer<'a> {
            bios_fns: BiosFns,
            buffer: &'a mut heapless::Vec<u8, 512>,
        }
        impl Write for Int10Writer<'_> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let mut str_to_write = s.as_bytes();
                while !str_to_write.is_empty() {
                    let bytes_to_copy = min(
                        str_to_write.len(),
                        self.buffer.capacity() - self.buffer.len(),
                    );
                    self.buffer
                        .extend_from_slice(&str_to_write[..bytes_to_copy])
                        .unwrap();
                    if bytes_to_copy == 0 {
                        self.bios_fns.print(self.buffer);
                        self.buffer.clear();
                    }
                    str_to_write = &str_to_write[bytes_to_copy..];
                }
                Ok(())
            }
        }
        let mut buffer = Default::default();
        let mut writer = WriterWithCr::new(Int10Writer {
            bios_fns: self.bios_fns,
            buffer: &mut buffer,
        });
        let msg = record.args();
        writeln!(writer, "{msg}").unwrap();
        if !buffer.is_empty() {
            self.bios_fns.print(&buffer);
        }
    }

    fn flush(&self) {}
}

pub fn init(bios_functions: BiosFns) {
    set_max_level(log::LevelFilter::Info);
    set_logger(DYN_LOGGER.call_once(|| {
        LOGGER.call_once(|| VesaConsoleLogger {
            bios_fns: bios_functions,
        })
    }))
    .unwrap();
}
