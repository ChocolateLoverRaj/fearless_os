use core::{
    num::NonZero,
    ptr::{NonNull, addr_of_mut},
};

use bitbybit::bitfield;
use common::SECTOR_1;
use zerocopy::{FromBytes, FromZeros, TryFromBytes};

#[repr(C)]
#[derive(Debug, TryFromBytes)]
struct Int15RawOutput {
    eax: u32,
    ebx: u32,
    carry_flag: bool,
    cl: u8,
    _padding: [u8; 6],
}

#[repr(C)]
struct UtilTable {
    int_10: extern "C" fn(u8),
    /// args: di, es, ebx, ecx
    int_15: extern "C" fn(u16, u16, u32, u32) -> Int15RawOutput,
}

fn table() -> &'static UtilTable {
    // Safety: previous boot stages created the util table at this addr
    unsafe {
        NonNull::new(usize::try_from(SECTOR_1).unwrap() as *mut UtilTable)
            .unwrap()
            .as_ref()
    }
}

pub fn int_10(byte: u8) {
    (table().int_10)(byte)
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
        let mut data = Int15RawData::new_zeroed();
        let data_addr = addr_of_mut!(data).addr();
        let es = u16::try_from(data_addr / 16).unwrap_or(u16::MAX);
        let di = u16::try_from(data_addr - usize::try_from(es).unwrap() * 16)
            .expect("cannot represent stack-allocated data pointer with real-mode addressing");
        let bx = entry_id;
        let ecx = 24;
        let output = (table().int_15)(di, es, bx, ecx);
        // let output: Int15RawOutput = try_transmute!(output).unwrap();
        if output.carry_flag {
            return Err(Int15Error::CarryFlagSet);
        }
        if output.eax != 0x534D4150 {
            return Err(Int15Error::InvalidEax(output.eax));
        }
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
