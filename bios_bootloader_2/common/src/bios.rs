use core::{
    num::NonZero,
    ptr::{NonNull, addr_of, addr_of_mut},
};

use bitbybit::bitfield;
use zerocopy::{FromBytes, FromZeros, transmute};

use crate::SECTOR_1;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Int15RawOutput {
    eax: u32,
    ebx: u32,
    carry_flag: bool,
    cl: u8,
    _padding: [u8; 6],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ExtendedReadRawOutput {
    carry_flag: bool,
    error: Option<NonZero<u8>>,
}

#[repr(C)]
struct UtilTable {
    low_mem_stack_pointer: Option<NonZero<u16>>,
    int_10: extern "C" fn(u8),
    /// args: di, es, ebx, ecx
    int_15: extern "C" fn(u32) -> Int15RawOutput,
    /// args: ds, si, dl
    extended_read: extern "C" fn(u16, u16, u8) -> ExtendedReadRawOutput,
    int_15_buffer: [u8; 24],
}

fn table() -> &'static UtilTable {
    // Safety: previous boot stages created the util table at this addr
    unsafe {
        NonNull::new(usize::try_from(SECTOR_1).unwrap() as *mut UtilTable)
            .unwrap()
            .as_ref()
    }
}

pub struct BiosFns {
    table: &'static UtilTable,
}

impl BiosFns {
    /// Safety: bios functions must exist at `SECTOR_1` and the bios stack pointer must be valid.
    pub unsafe fn new(bios_stack_pointer: Option<NonZero<u16>>) -> Self {
        let util_table = unsafe {
            NonNull::new(usize::try_from(SECTOR_1).unwrap() as *mut UtilTable)
                .unwrap()
                .as_mut()
        };
        util_table.low_mem_stack_pointer = bios_stack_pointer;

        Self { table: util_table }
    }

    pub fn int_10(&self, byte: u8) {
        (self.table.int_10)(byte)
    }
}

#[derive(Debug, Clone, Copy, FromBytes)]
#[repr(C)]
pub struct Int15RawData {
    pub base_addr: u64,
    pub len: u64,
    pub _type: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum Int15Error {
    CarryFlagSet,
    InvalidEax(u32),
}

#[bitfield(u32, debug)]
pub struct AcpiExtendedAttributes {
    #[bit(0, rw)]
    dont_ignore: bool,
    #[bit(1, rw)]
    non_volatile: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Int15Data {
    pub base_addr: u64,
    pub len: u64,
    pub _type: u32,
    pub acpi_extended_attributes: Option<AcpiExtendedAttributes>,
}

impl Int15Data {
    pub fn is_usable(&self) -> bool {
        self._type == 0x1
            && self
                .acpi_extended_attributes
                .is_none_or(|attributes| attributes.dont_ignore() && !attributes.non_volatile())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Int15Output {
    pub data: Int15Data,
    pub next_entry_index: Option<NonZero<u32>>,
}

pub fn int_15(entry_id: u32) -> Result<Int15Output, Int15Error> {
    Ok({
        let output = (table().int_15)(entry_id);
        // let output: Int15RawOutput = try_transmute!(output).unwrap();
        if output.carry_flag {
            return Err(Int15Error::CarryFlagSet);
        }
        if output.eax != 0x534D4150 {
            return Err(Int15Error::InvalidEax(output.eax));
        }
        let data: Int15RawData = transmute!(table().int_15_buffer);
        Int15Output {
            data: Int15Data {
                base_addr: data.base_addr,
                len: data.len,
                _type: data._type as u32,
                acpi_extended_attributes: match output.cl {
                    20 => None,
                    24 => Some(AcpiExtendedAttributes::new_with_raw_value(
                        (data._type >> 32) as u32,
                    )),
                    cl => panic!("Unexpected cl: {cl}"),
                },
            },
            next_entry_index: NonZero::new(output.ebx),
        }
    })
}

pub struct MemoryIterator {
    entry_id: Option<u32>,
}

impl Default for MemoryIterator {
    fn default() -> Self {
        Self { entry_id: Some(0) }
    }
}

impl Iterator for MemoryIterator {
    type Item = Result<Int15Data, Int15Error>;

    fn next(&mut self) -> Option<Self::Item> {
        Some({
            let entry_id = self.entry_id?;
            (|| {
                Ok({
                    let Int15Output {
                        data,
                        next_entry_index,
                    } = int_15(entry_id)?;
                    self.entry_id = next_entry_index.map(|id| id.get());
                    data
                })
            })()
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
pub struct ExtendedReadError {
    /// Technically this should always be `Some` but if the BIOS sets the carry flag while returning with ah = 0, this would be none.
    pub ah: Option<NonZero<u8>>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct DeviceAddressPacket {
    packet_size: u8,
    _reserved_0: u8,
    blocks_to_transfer: u8,
    _reserved_1: u8,
    host_buffer_address: RealModeAddr,
    starting_lba: u64,
}

/// # Safety
/// Overwrites data at the dest addr.
pub unsafe fn extended_read(
    disk: u8,
    src_lba: u64,
    dest_addr: RealModeAddr,
    blocks_to_transfer: u8,
) -> Result<(), ExtendedReadError> {
    assert!(blocks_to_transfer <= 127);
    let dap = DeviceAddressPacket {
        packet_size: size_of::<DeviceAddressPacket>().try_into().unwrap(),
        _reserved_0: 0,
        blocks_to_transfer,
        _reserved_1: 0,
        host_buffer_address: dest_addr,
        starting_lba: src_lba,
    };
    log::info!("DAP: {dap:#X?}");
    let dap_addr = RealModeAddr::try_from(u32::try_from(addr_of!(dap).addr()).unwrap()).unwrap();
    let ExtendedReadRawOutput { carry_flag, error } =
        (table().extended_read)(dap_addr.segment, dap_addr.offset, disk);
    if carry_flag || error.is_some() {
        return Err(ExtendedReadError { ah: error });
    }
    Ok(())
}
