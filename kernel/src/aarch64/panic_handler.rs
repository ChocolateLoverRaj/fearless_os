use aarch64_cpu::{
    asm::wfi,
    registers::{DAIF, Writeable},
};

use super::logger;

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    DAIF.set(0);
    unsafe { logger::force_unlock() };
    log::error!("{info}");
    loop {
        wfi();
    }
}
