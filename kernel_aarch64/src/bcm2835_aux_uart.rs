use core::ptr::NonNull;

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

    pub fn read_sync(&mut self) -> u8 {
        while !Lsr::new_with_raw_value(self.p.as_mut_ptr().lsr().read()).data_ready() {}
        self.p.as_mut_ptr().io().read() as u8
    }
}
