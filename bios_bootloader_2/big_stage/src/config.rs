use embedded_graphics::mono_font::{self, MonoFont};
use log::LevelFilter;

pub struct ScreenConfig {
    pub width: u16,
    pub height: u16,
    pub bpp: u8,
}

pub enum ScreenFlush {
    EveryLog,
    Manually,
}

pub struct Config {
    /// Useful for testing a specific resolution in virtual machines,
    /// or for using a smaller resolution in virtual machines so the window isn't too big.
    pub preffered_resolution: Option<ScreenConfig>,
    pub screen_log_level: LevelFilter,
    pub serial_log_level: LevelFilter,
    /// If true, will log to the screen and not UART, even if a UART is supported.
    /// This is useful for testing screen logging in a virtual machine.
    pub prefer_screen_logging: bool,
    pub font: MonoFont<'static>,
    /// Writing to the framebuffer can be slow, so manually flushing can save time.
    pub screen_flush: ScreenFlush,
    pub enter_acpi_mode: bool,
    /// On some hardware routing HPET interrupts to one of the supported I/O APIC irqs is not reliable,
    /// but enabling legacy replacement is reliable. The downside to legacy replacement is that the IRQs are
    /// fixed for 2 timers, but any additional timers still need to be configured.
    ///
    /// This setting only takes place if FSB interrupts are not supported by the HPET.
    /// FSB interrupts are better because they get directly sent to
    pub hpet_prefer_legacy_replacement: bool,
}

pub const CONFIG: Config = Config {
    preffered_resolution: Some(ScreenConfig {
        width: 1024,
        height: 768,
        bpp: 32,
    }),
    screen_log_level: LevelFilter::Info,
    serial_log_level: LevelFilter::Debug,
    prefer_screen_logging: false,
    font: mono_font::iso_8859_16::FONT_6X13,
    screen_flush: ScreenFlush::EveryLog,
    enter_acpi_mode: false,
    hpet_prefer_legacy_replacement: false,
};
