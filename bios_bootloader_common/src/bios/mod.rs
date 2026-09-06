pub mod disk;
pub mod memory;
pub mod real_mode_addr;
pub mod vesa;

use core::{num::NonZero, ptr::NonNull};

pub use self::disk::ExtendedReadError;
pub use self::real_mode_addr::RealModeAddr;
use crate::{
    SECTOR_1,
    bios::{
        disk::ExtendedReadFn,
        memory::Int15Fn,
        vesa::{PrintCharFn, VesaGetInfoFn, VesaGetModeFn, VesaSetModeFn},
    },
};

#[repr(C)]
struct UtilTable {
    low_mem_stack_pointer: Option<NonZero<u16>>,
    int_10: PrintCharFn,
    int_15: Int15Fn,
    extended_read: ExtendedReadFn,
    vesa_get_info: VesaGetInfoFn,
    vesa_get_mode: VesaGetModeFn,
    vesa_set_mode: VesaSetModeFn,
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
