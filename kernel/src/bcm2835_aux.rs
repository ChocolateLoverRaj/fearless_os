use core::ptr::NonNull;

use bitbybit::bitfield;
use volatile::{
    VolatileFieldAccess, VolatileRef,
    access::{ReadOnly, ReadWrite},
};

#[repr(C)]
#[derive(VolatileFieldAccess)]
pub struct Registers {
    #[access(ReadOnly)]
    irq: u32,
    #[access(ReadWrite)]
    enables: u32,
}

#[bitfield(u32, debug)]
struct Irq {
    #[bit(0, r)]
    mini_uart: bool,
    #[bit(1, r)]
    spi_1: bool,
    #[bit(2, r)]
    spi_2: bool,
}

#[bitfield(u32, debug)]
struct Enables {
    #[bit(0, rw)]
    mini_uart: bool,
    #[bit(1, rw)]
    spi_1: bool,
    #[bit(2, rw)]
    spi_2: bool,
}

pub struct Bcm2835Aux {
    p: VolatileRef<'static, Registers>,
}

impl Bcm2835Aux {
    pub unsafe fn new(ptr: NonNull<Registers>) -> Self {
        Self {
            p: unsafe { VolatileRef::new(ptr) },
        }
    }

    pub fn mini_uart_irq_pending(&self) -> bool {
        Irq::new_with_raw_value(self.p.as_ptr().irq().read()).mini_uart()
    }
}
