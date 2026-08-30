use core::{cmp::min, fmt::Write};

use common::{bios::BiosFns, writer_with_cr::WriterWithCr};

use crate::log_target::LogTarget;

pub struct VesaTextLogTarget {
    bios_fns: BiosFns,
    buffer: heapless::Vec<u8, 512>,
}

impl VesaTextLogTarget {
    pub fn new(bios_fns: BiosFns) -> Self {
        Self {
            bios_fns,
            buffer: Default::default(),
        }
    }
}

impl LogTarget for VesaTextLogTarget {
    fn write_with_color(&mut self, _color: crate::log_target::Color, msg: &dyn core::fmt::Display) {
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
        // Text mode doesn't support colors
        writeln!(writer, "{msg}").unwrap();
    }

    fn flush(&mut self) {
        self.bios_fns.print(&self.buffer);
        self.buffer.clear();
    }
}
