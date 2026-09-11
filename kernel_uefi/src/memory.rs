use core::ops::Range;

use force_send_sync::SendSync;
use kernel_common::{
    global_allocator::{KernelGlobalAllocator, new_global_allocator},
    initial_pmm::InitialFreeMem,
    memory::{self},
    vmm::VirtMemRange,
};
use static_cell::StaticCell;
use uefi::{
    boot::MemoryType,
    mem::memory_map::{MemoryMap, MemoryMapOwned},
};

const OFFSET_MAP_VIRT_ADDR: u64 = 0;
const OFFSET_MAP_LEN: u64 = 0x400000000000;
const DYNAMIC_VIRT_ADDR: u64 = 0x400000000000;
const DYNAMIC_VIRT_LEN: u64 = 0x400000000000;

static INITIAL_FREE_MEM: StaticCell<UefiInitialFreeMem> = StaticCell::new();

struct UefiInitialFreeMem {
    memory: SendSync<MemoryMapOwned>,
    index: usize,
}

impl InitialFreeMem for UefiInitialFreeMem {
    fn phys_end_addr(&self) -> u64 {
        let entry = self
            .memory
            .entries()
            .filter(|entry| entry.ty == MemoryType::CONVENTIONAL)
            .last()
            .unwrap();
        entry.phys_start + entry.page_count * 0x1000
    }
}

impl Iterator for UefiInitialFreeMem {
    type Item = Range<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.memory.len() {
            let entry = self.memory[self.index];
            self.index += 1;
            if entry.ty == MemoryType::CONVENTIONAL {
                return Some(entry.phys_start..entry.phys_start + entry.page_count * 0x1000);
            }
        }
        None
    }
}

pub unsafe fn init(memory: MemoryMapOwned) {
    let initial_free_mem = UefiInitialFreeMem {
        memory: unsafe { SendSync::new(memory) },
        index: 0,
    };
    let phys_mem_len = initial_free_mem.phys_end_addr();
    let initial_free_mem = INITIAL_FREE_MEM.init(initial_free_mem);
    unsafe {
        memory::init(
            initial_free_mem,
            VirtMemRange {
                addr: 0,
                len: phys_mem_len,
            },
            VirtMemRange {
                addr: DYNAMIC_VIRT_ADDR,
                len: DYNAMIC_VIRT_LEN,
            },
            VirtMemRange {
                addr: OFFSET_MAP_VIRT_ADDR,
                len: OFFSET_MAP_LEN,
            },
            None,
        )
    };
}

#[global_allocator]
static GLOBAL_ALLOCATOR: KernelGlobalAllocator =
    unsafe { new_global_allocator(OFFSET_MAP_VIRT_ADDR) };
