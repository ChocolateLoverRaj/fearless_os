use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct RealModeAddr {
    pub offset: u16,
    pub segment: u16,
}

#[derive(Debug)]
pub struct NotAddressableFromRealMode;

impl TryFrom<u32> for RealModeAddr {
    type Error = NotAddressableFromRealMode;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok({
            let segment = u16::try_from(value / 16).unwrap_or(u16::MAX);
            let offset = u16::try_from(value - u32::from(segment) * 16)
                .map_err(|_| NotAddressableFromRealMode)?;
            Self { segment, offset }
        })
    }
}

impl From<RealModeAddr> for u32 {
    fn from(value: RealModeAddr) -> Self {
        Self::from(value.segment) * 16 + Self::from(value.offset)
    }
}
