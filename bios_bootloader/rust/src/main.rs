#![no_std]
#![no_main]
mod logger;
mod writer_with_cr;

use core::{
    arch::naked_asm,
    mem,
    num::NonZero,
    panic::PanicInfo,
    ptr::{addr_of, addr_of_mut},
};

use zerocopy::{FromBytes, IntoBytes, TryFromBytes, transmute, try_transmute};

unsafe extern "C" {
    static __bss_start: *const u8;
    static __bss_u64s_to_copy: *const u8;
}

#[unsafe(naked)]
#[unsafe(link_section = ".text.start")]
#[unsafe(no_mangle)]
unsafe extern "C" fn _start() {
    naked_asm!(
        "
        // Zero the BSS
        xor rax, rax
        lea rdi, {__bss_start}
        lea rcx, {__bss_u64s_to_copy}
        rep stosq

        jmp {rust_start}
        ",
        __bss_start = sym __bss_start,
        __bss_u64s_to_copy = sym __bss_u64s_to_copy,
        rust_start = sym rust_start,
    )
}

type Int10 = extern "C" fn(u8);

#[derive(Debug, Clone, Copy, IntoBytes)]
#[repr(C)]
struct Int15RawOutput {
    rax: u64,
    rdx: u64,
}
type Int15 = unsafe extern "C" fn(u16, u32) -> Int15RawOutput;

#[derive(Debug, Clone, Copy, TryFromBytes)]
#[repr(C)]
struct Int15DecodedOutput {
    eax: u32,
    ebx: u32,
    cl: u8,
    carry_flag: bool,
    _padding: [u8; 6],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Int10Ptr(u16);

impl Int10Ptr {
    pub fn as_fn(&self) -> Int10 {
        // Safety: Self was created in the bootloader, which points to a valid 64-bit function with the signature
        unsafe { mem::transmute(self.0 as usize) }
    }
}

#[derive(Debug, Clone, Copy, FromBytes)]
#[repr(C)]
struct Int15Data {
    base_addr: u64,
    len: u64,
    _type: u64,
}

#[derive(Debug, Clone, Copy)]
struct Int15Output {
    pub base_addr: u64,
    pub len: u64,
    pub _type: u64,
    pub next_entry_index: Option<NonZero<u32>>,
}

#[derive(Debug, Clone, Copy)]
enum Int15Error {
    CarryFlagSet,
    InvalidEax,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Int15Ptr(u16);

impl Int15Ptr {
    pub fn call(&self, buffer: &mut [u8; 24], entry_index: u32) -> Result<Int15Output, Int15Error> {
        Ok({
            let int_15 = unsafe { mem::transmute::<_, Int15>(self.0 as usize) };
            let output =
                unsafe { int_15(u16::try_from(buffer.as_ptr().addr()).unwrap(), entry_index) };
            let output: Int15DecodedOutput = try_transmute!(output).unwrap();
            if output.carry_flag {
                return Err(Int15Error::CarryFlagSet);
            }
            if output.eax != 0x534D4150 {
                return Err(Int15Error::InvalidEax);
            }
            let Int15Data {
                base_addr,
                len,
                _type,
            } = transmute!(*buffer);
            Int15Output {
                base_addr,
                len,
                _type,
                next_entry_index: NonZero::new(output.ebx),
            }
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, TryFromBytes)]
struct ExtendedReadOutput {
    carry_flag: bool,
    ah: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, IntoBytes)]
struct ExtendedReadInput {
    device_address_packet: u16,
    disk: u8,
    _padding: u8,
}

type ExtendedReadFn = unsafe extern "C" fn(u32) -> u16;

#[derive(Debug)]
enum ExtendedReadError {
    CarryFlagSet(u8),
}

#[repr(C)]
struct ExtendedReadPtr(u16);

#[repr(C)]
struct DeviceAddressPacket {
    packet_size: u8,
    _reserved_0: u8,
    blocks_to_transfer: u8,
    _reserved_1: u8,
    host_buffer_address: u32,
    starting_lba: u64,
}

impl ExtendedReadPtr {
    pub fn call(
        &self,
        disk: u8,
        starting_lba: u64,
        dest: &mut [u8],
    ) -> Result<(), ExtendedReadError> {
        log::info!("dest: {dest:p}");
        let mut packet = DeviceAddressPacket {
            packet_size: size_of::<DeviceAddressPacket>().try_into().unwrap(),
            _reserved_0: 0,
            blocks_to_transfer: (dest.len() / 512).try_into().unwrap(),
            _reserved_1: 0,
            host_buffer_address: dest.as_ptr().addr().try_into().unwrap(),
            starting_lba,
        };
        let packet_addr = addr_of_mut!(packet).addr().try_into().unwrap();
        let input = ExtendedReadInput {
            device_address_packet: packet_addr,
            disk,
            _padding: Default::default(),
        };
        let extended_read = unsafe { mem::transmute::<_, ExtendedReadFn>(self.0 as usize) };
        let ret = unsafe { extended_read(transmute!(input)) };
        let output: ExtendedReadOutput = try_transmute!(ret).unwrap();
        if output.carry_flag {
            return Err(ExtendedReadError::CarryFlagSet(output.ah));
        }
        Ok(())
    }
}

#[repr(C)]
struct BootloaderTable {
    int_10: Int10Ptr,
    int_15: Int15Ptr,
    extended_read: ExtendedReadPtr,
    disk: u8,
    _padding: u8,
}

unsafe extern "C" fn rust_start(_: u64, _: u64, bootloader_table: &BootloaderTable, _: u64) -> ! {
    logger::init(bootloader_table.int_10);
    log::info!("Hello from Rust in long mode!");
    let mut buffer = [Default::default(); _];
    let mut entry_index = 0;
    loop {
        let output = bootloader_table
            .int_15
            .call(&mut buffer, entry_index)
            .unwrap();
        log::info!("{output:#X?}");
        let Some(next_entry_index) = output.next_entry_index else {
            break;
        };
        entry_index = next_entry_index.get();
    }
    log::info!("Done reading entries");
    let mut buffer = [Default::default(); 512];
    let disk = bootloader_table.disk;
    log::info!("Reading disk: {disk:#X}");
    bootloader_table
        .extended_read
        .call(bootloader_table.disk, 0, &mut buffer)
        .unwrap();
    log::info!("sector 0: {buffer:x?}");
    loop {}
}

#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    log::error!("{panic_info}");
    log::error!("---");
    loop {}
}
