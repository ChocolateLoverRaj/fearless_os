use core::{hint::spin_loop, ptr::NonNull};

use acpi::{AcpiTables, Handler, PciAddress, sdt::mcfg::Mcfg};
use alloc::collections::btree_map::BTreeMap;
use arbitrary_int::{u3, u5, u12};
use common::{OFFSET_MAP_VIRT_ADDR, paging::LeafMappingFlags, pat::STRONG_UNCACHEABLE_INDEX};
use ez_pci::{PciAccess, PciReadWriteValue, PcieInfo};
use spin::Mutex;
use x86_64::instructions::port::Port;

use crate::{hpet::HPET, memory::map_phys};

pub struct PcieData {
    pub info: PcieInfo,
    pub virt: u64,
}

pub static PCIE_MAPPINGS: Mutex<BTreeMap<u16, PcieData>> = Mutex::new(BTreeMap::new());

const ACPI_MAPPING_FLAGS: LeafMappingFlags = LeafMappingFlags {
    executable: false,
    writable: true,
    user_mode_accessible: false,
    pat_index: STRONG_UNCACHEABLE_INDEX,
};

#[derive(Clone)]
pub struct AcpiHandler;

impl AcpiHandler {
    fn read_pci<T: PciReadWriteValue>(&self, address: PciAddress, offset: u16) -> T {
        let mappings = PCIE_MAPPINGS.lock();
        let pcie_data = mappings.get(&address.segment()).unwrap();
        let mut pcie = unsafe {
            PciAccess::new_pcie(
                pcie_data.info,
                NonNull::slice_from_raw_parts(
                    NonNull::new(pcie_data.virt as *mut _).unwrap(),
                    SEGMENT_MAPPED_LEN.try_into().unwrap(),
                ),
            )
        };
        pcie.bus(address.bus())
            .device(u5::new(address.device()))
            .unwrap()
            .function(u3::new(address.function()))
            .unwrap()
            .read(u12::new(offset))
    }

    fn write_pci<T: PciReadWriteValue>(&self, address: PciAddress, offset: u16, value: T) {
        let mappings = PCIE_MAPPINGS.lock();
        let pcie_data = mappings.get(&address.segment()).unwrap();
        let mut pcie = unsafe {
            PciAccess::new_pcie(
                pcie_data.info,
                NonNull::slice_from_raw_parts(
                    NonNull::new(pcie_data.virt as *mut _).unwrap(),
                    SEGMENT_MAPPED_LEN.try_into().unwrap(),
                ),
            )
        };
        pcie.bus(address.bus())
            .device(u5::new(address.device()))
            .unwrap()
            .function(u3::new(address.function()))
            .unwrap()
            .write(u12::new(offset), value);
    }
}

impl Handler for AcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        let phys_addr = u64::try_from(physical_address).unwrap();
        let virt_start = map_phys(phys_addr, size.try_into().unwrap(), ACPI_MAPPING_FLAGS).unwrap();
        log::trace!("mapped {phys_addr:#X} len {size:#X}.");
        acpi::PhysicalMapping {
            handler: self.clone(),
            physical_start: physical_address,
            mapped_length: size,
            region_length: size,
            virtual_start: NonNull::new(virt_start as *mut _).unwrap(),
        }
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        // TODO: Maybe unmap?
    }

    fn read_u8(&self, address: usize) -> u8 {
        // FIXME: Ensure mapped
        let ptr = NonNull::new((OFFSET_MAP_VIRT_ADDR + u64::try_from(address).unwrap()) as *mut _)
            .unwrap();
        unsafe { ptr.read() }
    }

    fn read_u16(&self, address: usize) -> u16 {
        // FIXME: Ensure mapped
        let ptr = NonNull::new((OFFSET_MAP_VIRT_ADDR + u64::try_from(address).unwrap()) as *mut _)
            .unwrap();
        unsafe { ptr.read() }
    }

    fn read_u32(&self, address: usize) -> u32 {
        // FIXME: Ensure mapped
        let ptr = NonNull::new((OFFSET_MAP_VIRT_ADDR + u64::try_from(address).unwrap()) as *mut _)
            .unwrap();
        unsafe { ptr.read() }
    }

    fn read_u64(&self, address: usize) -> u64 {
        // FIXME: Ensure mapped
        let ptr = NonNull::new((OFFSET_MAP_VIRT_ADDR + u64::try_from(address).unwrap()) as *mut _)
            .unwrap();
        unsafe { ptr.read() }
    }

    fn write_u8(&self, address: usize, value: u8) {
        // FIXME: Ensure mapped
        let ptr = NonNull::new((OFFSET_MAP_VIRT_ADDR + u64::try_from(address).unwrap()) as *mut _)
            .unwrap();
        unsafe { ptr.write(value) }
    }

    fn write_u16(&self, address: usize, value: u16) {
        // FIXME: Ensure mapped
        let ptr = NonNull::new((OFFSET_MAP_VIRT_ADDR + u64::try_from(address).unwrap()) as *mut _)
            .unwrap();
        unsafe { ptr.write(value) }
    }

    fn write_u32(&self, address: usize, value: u32) {
        // FIXME: Ensure mapped
        let ptr = NonNull::new((OFFSET_MAP_VIRT_ADDR + u64::try_from(address).unwrap()) as *mut _)
            .unwrap();
        unsafe { ptr.write(value) }
    }

    fn write_u64(&self, address: usize, value: u64) {
        // FIXME: Ensure mapped
        let ptr = NonNull::new((OFFSET_MAP_VIRT_ADDR + u64::try_from(address).unwrap()) as *mut _)
            .unwrap();
        unsafe { ptr.write(value) }
    }

    fn read_io_u8(&self, port: u16) -> u8 {
        unsafe { Port::new(port).read() }
    }

    fn read_io_u16(&self, port: u16) -> u16 {
        unsafe { Port::new(port).read() }
    }

    fn read_io_u32(&self, port: u16) -> u32 {
        unsafe { Port::new(port).read() }
    }

    fn write_io_u8(&self, port: u16, value: u8) {
        unsafe { Port::new(port).write(value) }
    }

    fn write_io_u16(&self, port: u16, value: u16) {
        unsafe { Port::new(port).write(value) }
    }

    fn write_io_u32(&self, port: u16, value: u32) {
        unsafe { Port::new(port).write(value) }
    }

    fn read_pci_u8(&self, address: acpi::PciAddress, offset: u16) -> u8 {
        self.read_pci(address, offset)
    }

    fn read_pci_u16(&self, address: acpi::PciAddress, offset: u16) -> u16 {
        self.read_pci(address, offset)
    }

    fn read_pci_u32(&self, address: acpi::PciAddress, offset: u16) -> u32 {
        self.read_pci(address, offset)
    }

    fn write_pci_u8(&self, address: acpi::PciAddress, offset: u16, value: u8) {
        self.write_pci(address, offset, value)
    }

    fn write_pci_u16(&self, address: acpi::PciAddress, offset: u16, value: u16) {
        self.write_pci(address, offset, value)
    }

    fn write_pci_u32(&self, address: acpi::PciAddress, offset: u16, value: u32) {
        self.write_pci(address, offset, value)
    }

    fn nanos_since_boot(&self) -> u64 {
        todo!()
    }

    fn stall(&self, microseconds: u64) {
        let hpet = HPET.get().unwrap();
        let ticks_per_us = 1_000_000_000 / hpet.main_counter_tick_period();
        let ticks_to_sleep = microseconds * u64::from(ticks_per_us);
        let sleep_start_counter_value = hpet.main_counter_value();
        while hpet
            .main_counter_value()
            .wrapping_sub(sleep_start_counter_value)
            < ticks_to_sleep
        {
            spin_loop();
        }
    }

    fn sleep(&self, milliseconds: u64) {
        // TODO: Don't busy loop
        self.stall(milliseconds * 1000);
    }

    fn create_mutex(&self) -> acpi::Handle {
        // FIXME: Maybe we actually need a mutex?
        acpi::Handle(0)
    }

    fn acquire(&self, mutex: acpi::Handle, timeout: u16) -> Result<(), acpi::aml::AmlError> {
        // FIXME: Maybe we actually need a mutex?
        Ok(())
    }

    fn release(&self, mutex: acpi::Handle) {
        // FIXME: Maybe we actually need a mutex?
    }
}

pub const SEGMENT_MAPPED_LEN: u64 = 0x10000000;

pub fn init(tables: &AcpiTables<AcpiHandler>) {
    // Find MCFG
    let mcfg = tables.find_table::<Mcfg>().unwrap();
    log::debug!("MCFG: {:#X?}", mcfg.get());

    for entry in mcfg.entries() {
        log::trace!("Mapping MCFG entry: {:#X?}", entry);
        let virt_addr =
            map_phys(entry.base_address, SEGMENT_MAPPED_LEN, ACPI_MAPPING_FLAGS).unwrap();
        PCIE_MAPPINGS.lock().insert(
            entry.pci_segment_group,
            PcieData {
                info: PcieInfo {
                    bus_number_start: entry.bus_number_start,
                    bus_number_end: entry.bus_number_end,
                },
                virt: virt_addr,
            },
        );
    }
}
