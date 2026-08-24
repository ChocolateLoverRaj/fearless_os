use arbitrary_int::u3;
use x86_64::registers::model_specific::{Pat, PatMemoryType};

pub const OUR_PAT: [PatMemoryType; 8] = [
    // Keep the 4 as they would be without PAT
    PatMemoryType::WriteBack,
    PatMemoryType::WriteThrough,
    PatMemoryType::Uncacheable,
    PatMemoryType::StrongUncacheable,
    // This one is useful for framebuffer
    PatMemoryType::WriteCombining,
    // The rest we don't have a use for right now
    PatMemoryType::StrongUncacheable,
    PatMemoryType::StrongUncacheable,
    PatMemoryType::StrongUncacheable,
];

pub const WRITE_BACK_INDEX: u3 = u3::new(0);
pub const STRONG_UNCACHEABLE_INDEX: u3 = u3::new(3);
pub const WRITE_COMBINING_INDEX: u3 = u3::new(4);
