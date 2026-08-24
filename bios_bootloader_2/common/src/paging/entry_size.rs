/// How many bytes the entry and its children represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntrySize {
    _256T,
    _512G,
    _1G,
    _2M,
    _4K,
}
