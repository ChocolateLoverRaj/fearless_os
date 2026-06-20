use riscv::interrupt;

use super::logger;

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    interrupt::disable();
    unsafe { logger::force_unlock() };
    log::error!("{info}");
    loop {}
}
