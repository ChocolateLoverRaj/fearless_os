use spinning_top::RawSpinlock;
use talc::{DefaultBinning, TalcLock, min_first_heap_size, source::Claim};

const HEAP_SIZE: usize = 0x100000;

#[global_allocator]
pub static TALC: TalcLock<RawSpinlock, Claim> = TalcLock::new(unsafe {
    static mut INITIAL_HEAP: [u8; min_first_heap_size::<DefaultBinning>() + HEAP_SIZE] =
        [0; min_first_heap_size::<DefaultBinning>() + HEAP_SIZE];

    Claim::array(&raw mut INITIAL_HEAP)
});
