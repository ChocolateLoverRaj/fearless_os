use core::{fmt::Write, sync::atomic::AtomicBool};

use sbi::PhysicalAddress;

static TOOK_CONSOLE: AtomicBool = AtomicBool::new(false);

enum ConsoleMethod {
    LegacyConsole,
    Dbcn,
}

pub struct Console {
    method: ConsoleMethod,
}

impl Console {
    pub fn take() -> Option<Self> {
        if !TOOK_CONSOLE.swap(true, core::sync::atomic::Ordering::Relaxed) {
            Some(Self {
                method: if sbi::base::probe_extension(sbi::debug_console::EXTENSION_ID)
                    .is_available()
                {
                    ConsoleMethod::Dbcn
                } else {
                    ConsoleMethod::LegacyConsole
                },
            })
        } else {
            None
        }
    }
}

impl Write for Console {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match self.method {
            ConsoleMethod::LegacyConsole => {
                for &char in s.as_bytes() {
                    sbi::legacy::console_putchar(char);
                }
            }
            ConsoleMethod::Dbcn => {
                let mut total_bytes_written = 0;
                while total_bytes_written < s.len() {
                    let bytes_written = unsafe {
                        sbi::debug_console::write_ptr(PhysicalAddress::from_ptr(
                            core::ptr::from_ref(&s.as_bytes()[total_bytes_written..]).cast_mut(),
                        ))
                    }
                    .map_err(|_| core::fmt::Error)?;
                    total_bytes_written += bytes_written;
                    // bytes_written += self.write(s.as_bytes()).map_err(|e| core::fmt::Error)?;
                }
            }
        }
        Ok(())
    }
}
