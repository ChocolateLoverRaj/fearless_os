#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BigStageEntryInfo {
    pub low_used_mem_len: u64,
    pub big_stage_phys_start: u64,
}

pub type Entry = unsafe extern "C" fn(&BigStageEntryInfo) -> !;
