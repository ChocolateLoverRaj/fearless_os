use talc::{DefaultBinning, source::Source, sync::TalcLock};

use crate::memory::alloc_phys;

#[derive(Debug)]
pub struct TalcSource {
    offset_map_virt_addr: u64,
}

unsafe impl Source for TalcSource {
    fn acquire<B: talc::base::binning::Binning>(
        talc: &mut talc::base::Talc<Self, B>,
        layout: core::alloc::Layout,
    ) -> Result<(), ()> {
        let size = layout
            .size()
            .next_multiple_of(0x200000)
            .next_multiple_of(layout.align());
        let align = u64::try_from(layout.align()).unwrap().max(0x1000);

        log::info!("allocating {size:#X} with align {align:#X}.");
        let phy_start = alloc_phys(size.try_into().unwrap(), align)
            .ok_or(())
            .inspect_err(|_| log::warn!("failed to alloc phys mem for global allocator"))?;
        log::info!("allocated {phy_start:#X}.");

        unsafe {
            talc.claim(
                (talc.source.offset_map_virt_addr + phy_start) as *mut _,
                size,
            )
        };
        Ok(())
    }
}

pub type KernelGlobalAllocator =
    talc::sync::TalcLock<spinning_top::RawSpinlock, TalcSource, DefaultBinning>;

pub const unsafe fn new_global_allocator(offset_map_virt_start: u64) -> KernelGlobalAllocator {
    TalcLock::new(TalcSource {
        offset_map_virt_addr: offset_map_virt_start,
    })
}
