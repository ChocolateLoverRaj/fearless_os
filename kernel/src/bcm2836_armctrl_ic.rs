use core::ptr::NonNull;

use volatile::{VolatileFieldAccess, VolatileRef};

#[repr(C)]
#[derive(VolatileFieldAccess)]
pub struct Registers {
    irq_basic_pending: u32,
    irq_pending_1: u32,
    irq_pending_2: u32,
    fiq_control: u32,
    enable_irqs_1: u32,
    enable_irqs_2: u32,
    enable_basic_irqs: u32,
    disable_irqs_1: u32,
    disable_irqs_2: u32,
    disable_basic_irqs: u32,
}

pub struct Bcm2836ArmCtrlIc {
    p: VolatileRef<'static, Registers>,
}

impl Bcm2836ArmCtrlIc {
    pub unsafe fn new(ptr: NonNull<Registers>) -> Self {
        Self {
            p: unsafe { VolatileRef::new(ptr) },
        }
    }

    pub fn is_pending(&self, interrupt: u8) -> bool {
        if interrupt < 32 {
            self.p.as_ptr().irq_pending_1().read() & (1 << interrupt) != 0
        } else {
            self.p.as_ptr().irq_pending_2().read() & (1 << (interrupt - 32)) != 0
        }
    }

    pub fn get_interrupt_enabled(&self, interrupt: u8) -> bool {
        if interrupt < 32 {
            self.p.as_ptr().enable_irqs_1().read() & (1 << interrupt) != 0
        } else {
            self.p.as_ptr().enable_irqs_2().read() & (1 << (interrupt - 32)) != 0
        }
    }

    pub fn set_interrupt_enabled(&mut self, interrupt: u8, enabled: bool) {
        if enabled {
            if interrupt < 32 {
                self.p.as_mut_ptr().enable_irqs_1().write(1 << interrupt);
            } else {
                self.p
                    .as_mut_ptr()
                    .enable_irqs_2()
                    .write(1 << (interrupt - 32));
            }
        } else {
            if interrupt < 32 {
                self.p.as_mut_ptr().disable_irqs_1().write(1 << interrupt);
            } else {
                self.p
                    .as_mut_ptr()
                    .disable_irqs_2()
                    .write(1 << (interrupt - 32));
            }
        }
    }
}
