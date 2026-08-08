use core::{cmp::min, ptr::addr_of};

use bitmap_allocator::{BitAlloc, BitAlloc1M};
use common::{big_stage_api::BigStageEntryInfo, bios::MemoryIterator};
use spin::Mutex;

use crate::{__bss_end, __start};

struct StaticStuff {
    /// Enough to store up to 4 GiB point.
    /// Each bit represents a 4 KiB phys frame starting a phys addr 0.
    free_phys_mem: BitAlloc1M,
    /// Each bit represents a 4 KiB page starting at virt addr [`DYNAMIC_VIRT`].
    free_virt_mem: BitAlloc1M,
}

static MEMORY: Mutex<StaticStuff> = Mutex::new(StaticStuff {
    free_phys_mem: BitAlloc1M::DEFAULT,
    free_virt_mem: BitAlloc1M::DEFAULT,
});

pub unsafe fn init(info: &BigStageEntryInfo) {
    let mut s = MEMORY.lock();
    for a in MemoryIterator::default() {
        let a = a.unwrap();
        if a.is_usable() {
            let start = a.base_addr as usize;
            let end = start + a.len as usize;
            let start_page = start.next_multiple_of(0x1000) / 0x1000;
            let end_page = end / 0x1000;

            let range = start_page..min(end_page, BitAlloc1M::CAP);
            if !range.is_empty() {
                s.free_phys_mem.insert(range);
            }
        }
    }
    for used_range in [
        (0..info.low_used_mem_len),
        (info.big_stage_phys_start
            ..info.big_stage_phys_start
                + (addr_of!(__bss_end).addr() - addr_of!(__start).addr()) as u64),
    ] {
        let start_page = used_range.start / 0x1000;
        let end_page = used_range.end.next_multiple_of(0x1000) / 0x1000;
        let range = start_page as usize..end_page as usize;
        log::info!("removing range: {range:#X?}. Cap: {:#X}.", BitAlloc1M::CAP);
        s.free_phys_mem.remove(range);
    }

    s.free_virt_mem.insert(0..BitAlloc1M::CAP);
}
