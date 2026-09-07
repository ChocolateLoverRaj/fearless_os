use log::{LevelFilter, Log};
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

use crate::{log_color::LogColor, log_target::LogTarget};

pub trait LoggerInner {
    fn target_mut(&mut self) -> &mut dyn LogTarget;
    fn level_filter(&self) -> LevelFilter;
}

pub struct Logger<T> {
    data: Mutex<T>,
}

impl<T> Logger<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: Mutex::new(data),
        }
    }

    pub fn update<R, F: FnOnce(&mut T) -> R>(&self, f: F) -> R {
        without_interrupts(|| f(&mut *self.data.lock()))
    }
}

impl<T: LoggerInner + Send + Sync> Log for Logger<T> {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        without_interrupts(|| metadata.level() <= self.data.lock().level_filter())
    }

    fn log(&self, record: &log::Record) {
        without_interrupts(|| {
            let msg = record.args();
            let mut data = self.data.lock();
            let level = record.level();
            if level <= data.level_filter() {
                let target = data.target_mut();
                target.write_with_color(level.into(), &format_args!("{level:5} "));
                target.write_with_color(LogColor::Default, &format_args!("{msg}\n"));
                target.flush();
            }
        })
    }

    fn flush(&self) {
        without_interrupts(|| {
            self.data.lock().target_mut().flush();
        })
    }
}
