use core::{
    cmp::{max, min},
    fmt::Write,
};

use common::{bios::BiosFns, writer_with_cr::WriterWithCr};
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{Point, RgbColor},
};
use log::{Log, max_level, set_logger, set_max_level};
use spin::{Mutex, Once};
use uart_16550::{Uart16550Tty, backend::PioBackend};

use crate::{
    config::CONFIG, frame_buffer_embedded_graphics::FrameBufferEmbeddedGraphics,
    frame_buffer_writer::FrameBufferWriter,
};

/// Represents a color in a terminal or screen. The default color may depend on if the theme is light or dark.
enum Color {
    Default,
    BrightRed,
    BrightYellow,
    BrightBlue,
    BrightCyan,
    BrightMagenta,
}

enum LoggerKind {
    Console(BiosFns),
    Uart(Uart16550Tty<PioBackend>),
    FrameBuffer {
        frame_buffer: FrameBufferEmbeddedGraphics<'static>,
        position: Point,
    },
}

struct LoggerData {
    logger: LoggerKind,
}

impl LoggerData {}

struct Logger {
    data: Mutex<LoggerData>,
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= max_level()
    }

    fn log(&self, record: &log::Record) {
        let msg = record.args();
        let mut data = self.data.lock();
        match &mut data.logger {
            LoggerKind::Console(bios_fns) => {
                if record.level() >= CONFIG.screen_log_level {
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
                        bios_fns: *bios_fns,
                        buffer: &mut buffer,
                    });
                    writeln!(writer, "{msg}").unwrap();
                    if !buffer.is_empty() {
                        bios_fns.print(&buffer);
                    }
                }
            }
            LoggerKind::Uart(uart) => {
                writeln!(uart, "{msg}").unwrap();
            }
            LoggerKind::FrameBuffer {
                frame_buffer,
                position,
            } => {
                let mut writer = FrameBufferWriter {
                    display: frame_buffer,
                    position: position,
                    text_color: Rgb888::WHITE,
                    font: CONFIG.font,
                };
                writeln!(writer, "{msg}").unwrap();
                frame_buffer.flush();
            }
        }
    }

    fn flush(&self) {}
}

static LOGGER: Once<Logger> = Once::new();

pub fn init(bios_fns: BiosFns) {
    set_max_level(max(CONFIG.screen_log_level, CONFIG.serial_log_level));
    set_logger(LOGGER.call_once(|| Logger {
        data: Mutex::new(LoggerData {
            logger: LoggerKind::Console(bios_fns),
        }),
    }));
}

pub fn init_uart(uart: Uart16550Tty<PioBackend>) {
    LOGGER.get().unwrap().data.lock().logger = LoggerKind::Uart(uart);
}

/// Logs to the frame buffer instead of VBE text mode logging.
/// If specified, will do this even if logging to a UART was specified.
pub fn init_frame_buffer(
    frame_buffer: FrameBufferEmbeddedGraphics<'static>,
    replace_uart: bool,
) -> Option<FrameBufferEmbeddedGraphics<'static>> {
    let mut data = LOGGER.get().unwrap().data.lock();
    if !matches!(data.logger, LoggerKind::Uart(_)) || replace_uart {
        data.logger = LoggerKind::FrameBuffer {
            frame_buffer,
            position: Point::zero(),
        };
        None
    } else {
        Some(frame_buffer)
    }
}
