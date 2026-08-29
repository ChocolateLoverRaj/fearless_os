use core::num::NonZero;

use zerocopy::{FromBytes, IntoBytes, TryFromBytes, try_transmute};

use crate::bios::{BiosFns, RealModeAddr};

/// args: dl
pub(super) type ExtendedReadFn = fn(u8) -> u16;

#[repr(C)]
#[derive(Debug, Clone, Copy, TryFromBytes)]
pub(super) struct ExtendedReadRawOutput {
    carry_flag: bool,
    error: Option<NonZero<u8>>,
}

#[derive(Debug, Clone, Copy)]
pub struct ExtendedReadError {
    /// Technically this should always be `Some` but if the BIOS sets the carry flag while returning with ah = 0, this would be none.
    pub ah: Option<NonZero<u8>>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes)]
pub(super) struct DeviceAddressPacket {
    packet_size: u8,
    _reserved_0: u8,
    blocks_to_transfer: u8,
    _reserved_1: u8,
    host_buffer_address: RealModeAddr,
    starting_lba: u64,
}

impl BiosFns {
    /// # Safety
    /// Overwrites data at the dest addr.
    pub unsafe fn extended_read(
        &self,
        disk: u8,
        src_lba: u64,
        dest_addr: RealModeAddr,
        blocks_to_transfer: u8,
    ) -> Result<(), ExtendedReadError> {
        assert!(blocks_to_transfer <= 127);
        let device_address_packet = DeviceAddressPacket {
            packet_size: size_of::<DeviceAddressPacket>().try_into().unwrap(),
            _reserved_0: 0,
            blocks_to_transfer,
            _reserved_1: 0,
            host_buffer_address: dest_addr,
            starting_lba: src_lba,
        };
        log::debug!(
            "device_address_packet: {device_address_packet:#X?} at {:p}",
            &self.table().dap_buffer
        );
        self.table().dap_buffer = device_address_packet;
        let ExtendedReadRawOutput { carry_flag, error } =
            try_transmute!((self.table().extended_read)(disk)).unwrap();
        if carry_flag || error.is_some() {
            return Err(ExtendedReadError { ah: error });
        }
        Ok(())
    }
}
