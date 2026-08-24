use common::OFFSET_MAP_VIRT_ADDR;
use talc::{DefaultBinning, source::Source, sync::TalcLock};

use crate::memory::alloc_phys;

#[derive(Debug)]
struct TalcSource;

unsafe impl Source for TalcSource {
    fn acquire<B: talc::base::binning::Binning>(
        talc: &mut talc::base::Talc<Self, B>,
        layout: core::alloc::Layout,
    ) -> Result<(), ()> {
        let size = layout
            .size()
            .next_multiple_of(0x1000)
            .next_multiple_of(layout.align());
        let align = u64::try_from(layout.align()).unwrap().max(0x1000);

        let phy_start = alloc_phys(size.try_into().unwrap(), align).ok_or(())?;

        unsafe { talc.claim((OFFSET_MAP_VIRT_ADDR + phy_start) as *mut _, size) };
        Ok(())
    }
}

#[global_allocator]
static TALCK: talc::sync::TalcLock<spinning_top::RawSpinlock, TalcSource, DefaultBinning> =
    TalcLock::new(TalcSource);
