use core::num::NonZero;

use bitbybit::bitfield;
use zerocopy::{FromBytes, transmute};

use crate::bios::BiosFns;

/// args: di, es, ebx, ecx
pub(super) type Int15Fn = extern "C" fn(u32) -> Int15RawOutput;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct Int15RawOutput {
    eax: u32,
    ebx: u32,
    carry_flag: bool,
    cl: u8,
    _padding: [u8; 6],
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

impl BiosFns {
    pub fn int_15(&self, entry_id: u32) -> Result<Int15Output, Int15Error> {
        Ok({
            let output = (self.table().int_15)(entry_id);
            // let output: Int15RawOutput = try_transmute!(output).unwrap();
            if output.carry_flag {
                return Err(Int15Error::CarryFlagSet);
            }
            if output.eax != 0x534D4150 {
                return Err(Int15Error::InvalidEax(output.eax));
            }
            let data: Int15RawData = transmute!(self.table().int_15_buffer);
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

    pub fn memory(&self) -> MemoryIterator<'_> {
        MemoryIterator {
            bios_fns: self,
            entry_id: Some(0),
        }
    }
}

pub struct MemoryIterator<'a> {
    bios_fns: &'a BiosFns,
    entry_id: Option<u32>,
}

impl Iterator for MemoryIterator<'_> {
    type Item = Result<Int15Data, Int15Error>;

    fn next(&mut self) -> Option<Self::Item> {
        Some({
            let entry_id = self.entry_id?;
            (|| {
                Ok({
                    let Int15Output {
                        data,
                        next_entry_index,
                    } = self.bios_fns.int_15(entry_id)?;
                    self.entry_id = next_entry_index.map(|id| id.get());
                    data
                })
            })()
        })
    }
}
