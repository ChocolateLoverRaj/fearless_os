pub mod disk;
pub mod memory;
pub mod vesa;

use core::{num::NonZero, ptr::NonNull};

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

pub use self::disk::ExtendedReadError;
use crate::{
    SECTOR_1,
    bios::{
        disk::ExtendedReadFn,
        memory::Int15Fn,
        vesa::{PrintCharFn, VesaGetInfoFn},
    },
};

#[repr(C)]
struct UtilTable {
    low_mem_stack_pointer: Option<NonZero<u16>>,
    int_10: PrintCharFn,
    int_15: Int15Fn,
    extended_read: ExtendedReadFn,
    vesa_get_info: VesaGetInfoFn,
    buffer: [u8; 512],
}

#[derive(Clone, Copy)]
pub struct BiosFns {
    table: NonNull<UtilTable>,
}

unsafe impl Send for BiosFns {}
unsafe impl Sync for BiosFns {}

impl BiosFns {
    fn table(&self) -> &'static mut UtilTable {
        unsafe { self.table.clone().as_mut() }
    }

    /// Safety: bios functions must exist at `SECTOR_1` and the bios stack pointer must be valid.
    /// You must only call this from one thread.
    pub unsafe fn new(bios_stack_pointer: Option<NonZero<u16>>) -> Self {
        let mut util_table =
            NonNull::new(usize::try_from(SECTOR_1).unwrap() as *mut UtilTable).unwrap();
        {
            let table = unsafe { util_table.as_mut() };
            table.low_mem_stack_pointer = bios_stack_pointer;
        }

        Self { table: util_table }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct RealModeAddr {
    offset: u16,
    segment: u16,
}

#[derive(Debug)]
pub struct NotAddressableFromRealMode;

impl TryFrom<u32> for RealModeAddr {
    type Error = NotAddressableFromRealMode;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok({
            let segment = u16::try_from(value / 16).unwrap_or(u16::MAX);
            let offset = u16::try_from(value - u32::from(segment) * 16)
                .map_err(|_| NotAddressableFromRealMode)?;
            Self { segment, offset }
        })
    }
}
