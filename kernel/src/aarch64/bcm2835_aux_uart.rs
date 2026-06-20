use core::{fmt::Write, ptr::NonNull};

use bitbybit::bitfield;
use volatile::{
    VolatileFieldAccess, VolatileRef,
    access::{ReadOnly, ReadWrite},
};

#[repr(C)]
#[derive(VolatileFieldAccess)]
pub struct UartRegs {
    #[access(ReadWrite)]
    io: u32,
    ier: u32,
    iir: u32,
    lcr: u32,
    mcr: u32,
    #[access(ReadOnly)]
    lsr: u32,
    msr: u32,
    scratch: u32,
    cntl: u32,
    stat: u32,
    baud: u32,
}

#[bitfield(u32, debug)]
struct Lsr {
    /// No pending data to transmit
    #[bit(6, r)]
    transmitter_idle: bool,
    /// There is space in the FIFO to queue up at least 1 byte to transmit
    #[bit(5, r)]
    transmitter_empty: bool,
    #[bit(1, r)]
    receiver_overrun: bool,
    /// There is at least 1 byte ready to be read from the FIFO
    #[bit(0, r)]
    data_ready: bool,
}

#[bitfield(u32, debug)]
struct Ier {
    #[bit(0, rw)]
    rx_enabled: bool,
    #[bit(1, rw)]
    tx_enabled: bool,
}

#[bitfield(u32, debug)]
struct Iir {
    #[bit(0, r)]
    interrupt_not_pending: bool,
    #[bit(1, r)]
    tx: bool,
    #[bit(2, r)]
    rx: bool,
}

pub struct Bcm2835AuxUart {
    p: VolatileRef<'static, UartRegs>,
}

impl Bcm2835AuxUart {
    pub unsafe fn new(ptr: NonNull<UartRegs>) -> Self {
        Self {
            p: unsafe { VolatileRef::new(ptr) },
        }
    }

    pub fn write_sync_no_flush(&mut self, byte: u8) {
        self.p.as_mut_ptr().io().write(byte.into());
        while !Lsr::new_with_raw_value(self.p.as_mut_ptr().lsr().read()).transmitter_empty() {}
    }

    pub fn try_read(&mut self) -> Option<u8> {
        if Lsr::new_with_raw_value(self.p.as_mut_ptr().lsr().read()).data_ready() {
            Some(self.p.as_mut_ptr().io().read() as u8)
        } else {
            None
        }
    }

    pub fn read_blocking(&mut self) -> u8 {
        loop {
            if let Some(byte) = self.try_read() {
                break byte;
            }
        }
    }

    pub fn interrupt_pending(&self) -> bool {
        !Iir::new_with_raw_value(self.p.as_ptr().iir().read()).interrupt_not_pending()
    }

    pub fn rx_interrupt_pending(&self) -> bool {
        Iir::new_with_raw_value(self.p.as_ptr().iir().read()).rx()
    }

    pub fn tx_interrupt_pending(&self) -> bool {
        Iir::new_with_raw_value(self.p.as_ptr().iir().read()).tx()
    }

    pub fn rx_interrupt_enabled(&self) -> bool {
        Ier::new_with_raw_value(self.p.as_ptr().ier().read()).rx_enabled()
    }

    pub fn tx_interrupt_enabled(&self) -> bool {
        Ier::new_with_raw_value(self.p.as_ptr().ier().read()).tx_enabled()
    }

    pub fn set_rx_interrupt_enabled(&mut self, enabled: bool) {
        self.p.as_mut_ptr().ier().update(|ier| {
            let mut ier = Ier::new_with_raw_value(ier);
            ier.set_rx_enabled(enabled);
            ier.raw_value
        });
    }

    pub fn set_tx_interrupt_enabled(&mut self, enabled: bool) {
        self.p.as_mut_ptr().ier().update(|ier| {
            let mut ier = Ier::new_with_raw_value(ier);
            ier.set_tx_enabled(enabled);
            ier.raw_value
        });
    }
}

impl Write for Bcm2835AuxUart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for &byte in s.as_bytes() {
            self.write_sync_no_flush(byte);
        }
        Ok(())
    }
}
