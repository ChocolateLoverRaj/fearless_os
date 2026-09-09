use raw_cpuid::CpuId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopLevel {
    /// Top table when using 5-level paging. Maps 128 PiB.
    Maps128P,
    /// Top table when using 4-level paging. Maps 256 TiB.
    Maps256T,
}

impl TopLevel {
    pub fn max_supported() -> Self {
        if CpuId::new()
            .get_extended_feature_info()
            .is_some_and(|info| info.has_la57())
        {
            TopLevel::Maps128P
        } else {
            TopLevel::Maps256T
        }
    }
}
