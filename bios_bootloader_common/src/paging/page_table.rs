use core::ops::{Index, IndexMut};

use arbitrary_int::u9;

#[repr(C, align(0x1000))]
#[derive(Debug, Clone, Copy)]
pub struct PageTable([u64; 512]);

impl PageTable {
    pub const fn new() -> Self {
        Self([0; 512])
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<u9> for PageTable {
    type Output = u64;

    fn index(&self, index: u9) -> &Self::Output {
        &self.0[usize::try_from(index.value()).unwrap()]
    }
}

impl IndexMut<u9> for PageTable {
    fn index_mut(&mut self, index: u9) -> &mut Self::Output {
        &mut self.0[usize::try_from(index.value()).unwrap()]
    }
}
