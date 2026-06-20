#![no_std]
#![no_main]

mod console;
mod logger;
mod paging;
mod panic_handler;
mod start;
mod trap;

use core::{
    arch::naked_asm,
    ops::{Deref, DerefMut},
    ptr::{NonNull, addr_of},
};

use arbitrary_int::u44;
use fdt::Fdt;
use log::info;
use riscv::{
    asm::wfi,
    interrupt::{self},
    register::{
        satp::{self},
        sie::{self},
        stvec::{self, Stvec, TrapMode},
        time,
    },
};
use sbi::{
    PhysicalAddress,
    hsm::{HartState, SuspendType},
};
use spin::Mutex;

use self::{
    paging::{PageTable, map_page},
    trap::trap_entry,
};

// These variables are defined in the linker script
unsafe extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
    static __bss_start: u8;
    static __bss_end: u8;
    static __stack_top: u8;
}

// #[unsafe(naked)]
// unsafe extern "C" fn other_hart_start() -> ! {
//     naked_asm!(
//         "
//         lla sp, __stack_top
//         li t0, 64 * 1024      # 32 KiB, 1/4 of reserved stack space
//         mul t0, t0, a1        # t0 = stack_number * stack_size
//         sub sp, sp, t0
//         j {rust}
//         ",
//         rust = sym other_hart_main
//     )
// }

/// Currently assumes Sv57 paging with just one root page table
static PAGE_TABLE: Mutex<PageTable> = Mutex::new(PageTable::new());

extern "C" fn kernel_main(hart_id: usize, fdt_addr: usize) -> ! {
    unsafe { logger::init() }
    log::info!("Kernel is logging! HART ID: {hart_id}. *mut FDT = {fdt_addr:#X}");
    unsafe {
        stvec::write(Stvec::new(
            trap_entry as *const () as usize,
            TrapMode::Direct,
        ))
    };
    log::debug!("Initialized Interrupts");
    // Enable paging
    let fdt = unsafe { Fdt::from_ptr(fdt_addr as *const _) }.unwrap();
    let mmu_type = fdt
        .cpus()
        .find(|cpu| cpu.ids().first() == hart_id)
        .unwrap()
        .property("mmu-type")
        .unwrap()
        .as_str()
        .unwrap();
    // FIXME: Use actual available memory regions and don't just guess
    let mut memory = addr_of!(__stack_top).addr();
    // TODO: Support Sv39 and Sv48 too
    if mmu_type == "riscv,sv57" {
        let mut page_table = PAGE_TABLE.try_lock().unwrap();
        let kernel_addr = addr_of!(__kernel_start).addr();
        let kernel_end_addr = addr_of!(__kernel_end).addr();
        let kernel_start_ppn = (kernel_addr as u64) >> 12;
        let kernel_end_ppn = (kernel_end_addr as u64 - 1) >> 12;
        info!("kernel ppn: {:#X?}", kernel_start_ppn..=kernel_end_ppn);
        for kernel_ppn in kernel_start_ppn..=kernel_end_ppn {
            info!("Mapping kernel ppn: {:#X}", kernel_ppn);
            unsafe {
                map_page(
                    kernel_ppn << 12,
                    u44::new(kernel_ppn),
                    NonNull::from_mut(page_table.deref_mut()),
                    || {
                        let addr = memory.next_multiple_of(0x1000);
                        memory = addr + 0x1000;
                        u44::new(addr as u64 >> 12)
                    },
                )
            };
        }
        // Leave guard page unmapped
        let stack_addr = addr_of!(__kernel_end).addr() + 0x1000;
        let stack_start_ppn = stack_addr as u64 >> 12;
        let stack_end_ppn = stack_start_ppn + 4;
        for stack_ppn in stack_start_ppn..stack_end_ppn {
            info!("Mapping stack: {stack_ppn:#X}");
            unsafe {
                map_page(
                    stack_ppn << 12,
                    u44::new(stack_ppn),
                    NonNull::from_mut(page_table.deref_mut()),
                    || {
                        let addr = memory.next_multiple_of(0x1000);
                        memory = addr + 0x1000;
                        u44::new(addr as u64 >> 12)
                    },
                )
            };
        }
        // Map the FDT
        let fdt_ppn_start = fdt_addr as u64 >> 12;
        let fdt_ppn_end_inclusive = (fdt_addr as u64 + fdt.total_size() as u64 - 1) >> 12;
        for fdt_ppn in fdt_ppn_start..=fdt_ppn_end_inclusive {
            info!("Mapping FDT ppn: {:#X}", fdt_ppn);
            unsafe {
                map_page(
                    fdt_ppn << 12,
                    u44::new(fdt_ppn),
                    NonNull::from_mut(page_table.deref_mut()),
                    || {
                        let addr = memory.next_multiple_of(0x1000);
                        memory = addr + 0x1000;
                        u44::new(addr as u64 >> 12)
                    },
                )
            };
        }
        let page_table_ppn = core::ptr::from_ref(page_table.deref()).addr() >> 12;
        info!("Enabling Sv57");
        unsafe { satp::set(satp::Mode::Sv57, 0, page_table_ppn) };
    } else {
        panic!("Unknown mmu type: {mmu_type:?}");
    }

    if sbi::base::probe_extension(sbi::hsm::EXTENSION_ID).is_available() {
        let mut hart_id = 0;
        let mut stack_number = 1; // our stack number is 0
        while let Ok(state) = sbi::hsm::hart_state(hart_id) {
            log::info!("Hart {hart_id} state: {state:?}");
            if state == HartState::Stopped {
                // unsafe {
                //     sbi::hsm::hart_start(
                //         hart_id,
                //         PhysicalAddress::new(other_hart_start as *const () as usize),
                //         stack_number,
                //     )
                // }
                // .unwrap();
                stack_number += 1;
            }
            hart_id += 1;
        }
    }
    unsafe { interrupt::enable() };

    if sbi::base::probe_extension(sbi::timer::EXTENSION_ID).is_available() {
        let time = time::read64();
        sbi::timer::set_timer(time + 10000).unwrap();
    }

    unsafe { sie::set_stimer() };

    loop {
        log::info!(".");
        idle();
    }
}

fn idle() {
    if sbi::base::probe_extension(sbi::hsm::EXTENSION_ID).is_available() {
        unsafe {
            sbi::hsm::hart_suspend(SuspendType::DefaultNonRetentive {
                resume_address: PhysicalAddress::new(resume_entry as *const () as usize),
                opaque: 67,
            })
        }
        .unwrap();
    } else {
        wfi();
    }
}

#[unsafe(naked)]
unsafe extern "C" fn resume_entry() {
    naked_asm!(
        "
        lla sp, __stack_top
        j {rust}
        ",
        rust = sym resume_main
    );
}

unsafe extern "C" fn resume_main(hart_id: usize, extra_input: usize) -> ! {
    log::info!("hart {hart_id} resumed from suspend with extra input {extra_input}");
    unsafe {
        stvec::write(Stvec::new(
            trap_entry as *const () as usize,
            TrapMode::Direct,
        ))
    };
    unsafe { interrupt::enable() };
    unsafe { sie::set_stimer() };
    loop {
        log::info!(".");
        idle();
    }
}

unsafe extern "C" fn other_hart_main(hart_id: usize, extra_input: usize) -> ! {
    log::info!("Hello from hart {hart_id} with extra input {extra_input}");

    sbi::hsm::hart_stop().unwrap();
    unreachable!()
}
