use arbitrary_int::u3;

#[derive(Debug, Clone, Copy)]
pub struct LeafMappingFlags {
    pub writable: bool,
    pub user_mode_accessible: bool,
    pub executable: bool,
    pub pat_index: u3,
}
