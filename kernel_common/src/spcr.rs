use core::ptr::NonNull;

use acpi::{
    AcpiError,
    address::{AddressSpace, StandardAccessSize},
    sdt::spcr::{Spcr, SpcrInterfaceType},
};
use uart_16550::{
    BaudRate, Config, Uart16550Tty, Uart16550TtyError,
    backend::{MmioAddress, MmioBackend},
};

use crate::{
    memory::{MapPhysError, map_phys},
    paging::LeafMappingFlags,
    pat::STRONG_UNCACHEABLE_INDEX,
};

pub fn init(spcr: &Spcr) -> Result<Uart16550Tty<MmioBackend>, Error> {
    log::debug!("SPCR found: {:X?}", &*spcr);
    if !matches!(
        spcr.interface_type(),
        SpcrInterfaceType::Full16550
            | SpcrInterfaceType::Full16450
            | SpcrInterfaceType::Generic16550
    ) {
        return Err(Error::UnknownInterfaceType(spcr.interface_type()));
    }
    let base_generic_addr = spcr
        .base_address()
        .ok_or(Error::NoBaseAddr)?
        .map_err(Error::BaseAddrError)?;

    if base_generic_addr.address_space != AddressSpace::SystemMemory {
        return Err(Error::UnexpectedAddrSpace(base_generic_addr.address_space));
    }
    let stride = match base_generic_addr.standard_access_size().unwrap() {
        StandardAccessSize::ByteAccess => 1_u8,
        StandardAccessSize::WordAccess => 2,
        StandardAccessSize::DWordAccess => 4,
        StandardAccessSize::QWordAccess => 8,
        StandardAccessSize::Undefined => panic!("Unknown access size"),
    };
    let map_len = 8 * u64::from(stride);
    let phys_addr = base_generic_addr.address;
    let virt_addr = NonNull::new(
        map_phys(
            phys_addr,
            map_len,
            LeafMappingFlags {
                writable: true,
                executable: false,
                user_mode_accessible: false,
                pat_index: STRONG_UNCACHEABLE_INDEX,
            },
        )
        .map_err(Error::Map)? as *mut _,
    )
    .expect("addr is non-zero");
    log::debug!("Serial port phys: {phys_addr:#X}, virt: {virt_addr:?}, stride: {stride:#X}");
    let config = Config {
        baud_rate: BaudRate::Baud115200,
        ..Default::default()
    };
    let uart =
        unsafe { Uart16550Tty::new_mmio(virt_addr, stride, config) }.map_err(Error::UartInit)?;
    return Ok(uart);
}

#[derive(Debug)]
pub enum Error {
    UnknownInterfaceType(SpcrInterfaceType),
    NoBaseAddr,
    BaseAddrError(AcpiError),
    Map(MapPhysError),
    UnexpectedAddrSpace(AddressSpace),
    UartInit(Uart16550TtyError<MmioAddress>),
}
