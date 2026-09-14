use core::fmt::Write;

use alloc::boxed::Box;
use ez_pci::{
    BarWithSize, CLASS_SIMPLE_COMMUNICATION_CONTROLLER, IoBarInfo, PROG_IF_16550_COMPATIBLE_0,
    PROG_IF_16550_COMPATIBLE_1, SUBCLASS_SERIAL_CONTROLLER,
};
use uart_16550::{Uart16550Tty, Uart16550TtyError, backend::PortIoAddress};

pub fn init(
    mut function: ez_pci::PciFunction,
) -> Result<Option<Box<dyn Write + Send>>, Uart16550TtyError<PortIoAddress>> {
    if function.class_code() != CLASS_SIMPLE_COMMUNICATION_CONTROLLER
        || function.sub_class() != SUBCLASS_SERIAL_CONTROLLER
    {
        return Ok(None);
    }
    let prog_if = function.prog_if();
    if !(prog_if == PROG_IF_16550_COMPATIBLE_0 || prog_if == PROG_IF_16550_COMPATIBLE_1) {
        log::warn!(
            "Serial port PCI function exists but we have no driver for it. Prog IF: {prog_if:#X}"
        );
        return Ok(None);
    }
    log::debug!("Found serial port PCI function. Prog IF: {prog_if:#X}");
    let bar = match function.read_bar_with_size(0) {
        Ok(Some(bar)) => bar,
        Ok(None) => {
            log::error!("BAR 0 not present");
            return Ok(None);
        }
        Err(e) => {
            log::error!("Failed to read BAR 0: {e:?}");
            return Ok(None);
        }
    };
    match bar {
        BarWithSize::Io(IoBarInfo { addr, size: _ }) => {
            let uart =
                unsafe { Uart16550Tty::new_port(addr.try_into().unwrap(), Default::default()) }?;
            return Ok(Some(Box::new(uart)));
        }
        BarWithSize::Memory(_) => todo!(),
    }
}
