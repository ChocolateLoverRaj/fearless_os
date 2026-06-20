use core::{mem::MaybeUninit, ptr::NonNull};

use arbitrary_int::{u2, u44};
use bitbybit::bitfield;

#[bitfield(u64, default = 0, debug)]
pub struct Pte {
    #[bit(0, rw)]
    v: bool,
    #[bit(1, rw)]
    r: bool,
    #[bit(2, rw)]
    w: bool,
    #[bit(3, rw)]
    x: bool,
    #[bit(4, rw)]
    u: bool,
    #[bit(5, rw)]
    g: bool,
    #[bit(6, rw)]
    a: bool,
    #[bit(7, rw)]
    d: bool,
    #[bits(10..=53, rw)]
    ppn: u44,
    #[bits(61..=62, rw)]
    pbmt: u2,
    #[bit(63, rw)]
    n: bool,
}

#[repr(C, align(0x1000))]
pub struct PageTable {
    pub entries: [u64; 512],
}

impl PageTable {
    pub const fn new() -> Self {
        Self { entries: [0; _] }
    }
}

/// Assumes paging is disabled
pub unsafe fn map_page(
    virt: u64,
    phys: u44,
    root_page_table: NonNull<PageTable>,
    mut alloc_page: impl FnMut() -> u44,
) {
    let mut page_table = root_page_table;
    // Levels are 4->3->2->1->0
    let mut level = 4;
    loop {
        let entry_index = match level {
            4 => (virt >> 48) & 0b1111_1111,
            3 => (virt >> 39) & 0b1_1111_1111,
            2 => (virt >> 30) & 0b1_1111_1111,
            1 => (virt >> 21) & 0b1_1111_1111,
            0 => (virt >> 12) & 0b1_1111_1111,
            _ => unreachable!(),
        } as usize;

        let page_table_ptr = unsafe { page_table.as_mut() };
        let entry = &mut page_table_ptr.entries[entry_index];
        if level == 0 {
            // info!("Mapping {virt:#X} -> {phys:#X}");
            *entry = Pte::default()
                .with_v(true)
                .with_r(true)
                .with_w(true)
                .with_x(true)
                .with_ppn(phys)
                .raw_value;
            break;
        }

        if !Pte::new_with_raw_value(*entry).v() {
            let child_page_table_ppn = alloc_page();
            // info!(
            //     "Creating new L{} table using ppn {:#X}",
            //     level - 1,
            //     child_page_table_ppn
            // );
            let child_page_table =
                NonNull::new((child_page_table_ppn.value() << 12) as *mut MaybeUninit<PageTable>)
                    .unwrap();
            // child_page_table.write(PageTable::new());
            // info!("Writing to {child_page_table:p}");
            unsafe { child_page_table.write_bytes(0, 1) };
            *entry = Pte::default()
                .with_v(true)
                .with_ppn(child_page_table_ppn)
                .raw_value;
        }
        page_table =
            NonNull::new((Pte::new_with_raw_value(*entry).ppn().value() << 12) as *mut PageTable)
                .unwrap();
        level -= 1;
    }
}
