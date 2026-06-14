#![no_std]
#![no_main]

mod console;
mod logger;
mod paging;

use core::{
    arch::naked_asm,
    ops::{Deref, DerefMut, RangeInclusive},
    ptr::{NonNull, addr_of},
};

use arbitrary_int::u44;
use fdt::{Fdt, node::MemoryReservation, standard_nodes::MemoryRegion};
use log::info;
use riscv::{
    asm::wfi,
    interrupt::{self, Interrupt},
    register::{
        satp::{self, Satp},
        scause, sepc,
        sie::{self, Sie},
        sip,
        sstatus::{self, Sstatus},
        stval,
        stvec::{self, Stvec, TrapMode},
        time,
    },
};
use sbi::{
    PhysicalAddress,
    hsm::{HartState, SuspendType},
};
use spin::Mutex;

use crate::paging::{PageTable, map_page};

// These variables are defined in the linker script
unsafe extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
    static __bss_start: u8;
    static __bss_end: u8;
    static __stack_top: u8;
}

/// OpenSBI passes the HART ID in the `a0` register and a pointer to the device tree in the `a1`
/// register. Since we don't modify those registers, we can just jump to `kernel_main` and those
/// two inputs will be passed to it.
#[unsafe(link_section = ".text._header")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "C" fn _start() {
    naked_asm!(
        "
            j {start}
        ",
        start = sym start
    )
}
#[unsafe(naked)]
unsafe extern "C" fn start() {
    naked_asm!(
        "
        lla t0, _start

        // Do relocations
        lla t1, __rel_start
        lla t2, __rel_end
        .reloc_loop:
            beq t1, t2, .reloc_loop_done

            // Load the relocation type
            // It should be R_RISCV_RELATIVE
            // The lower 32 bytes store it
            lwu t3, 8(t1)
            li t4, 3
            bne t3, t4, .unknown_reloc

            // Load the default offset
            ld t4, 16(t1)
            // Add the load offset
            add t4, t4, t0

            // Get a pointer to the location in memory we need to modify
            ld t5, (t1)
            // Adjust the pointer itself for the offset
            add t5, t5, t0

            // Write to it
            sd t4, (t5)

            // Continue to the next relocation
            add t1, t1, 24
            j .reloc_loop

        .unknown_reloc:
            j .unknown_reloc

        .reloc_loop_done:

        // Set the stack pointer
        lla sp, __stack_top

        // Zero the BSS
        lla t1, __bss_start
        lla t2, __bss_end
        .zero_bss_loop:
            beq t1, t2, .zero_bss_loop_done
            sd zero, (t1)
            add t1, t1, 8
            j .zero_bss_loop

        .zero_bss_loop_done:

        j {kernel_main}
        ",
        kernel_main = sym kernel_main
    )
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
            kernel_entry as *const () as usize,
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
            kernel_entry as *const () as usize,
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

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    interrupt::disable();
    unsafe { logger::force_unlock() };
    log::error!("{info}");
    loop {}
}

#[repr(C)]
pub struct TrapFrame {
    pub ra: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub sp: usize,
}

#[unsafe(naked)]
extern "C" fn kernel_entry() {
    naked_asm!(
        "
            .align 4
            // .a:
            //     j .a
            addi sp, sp, -256

            sd ra,  8 * 0(sp)
            sd gp,  8 * 1(sp)
            sd tp,  8 * 2(sp)
            sd t0,  8 * 3(sp)
            sd t1,  8 * 4(sp)
            sd t2,  8 * 5(sp)
            sd t3,  8 * 6(sp)
            sd t4,  8 * 7(sp)
            sd t5,  8 * 8(sp)
            sd t6,  8 * 9(sp)
            sd a0,  8 * 10(sp)
            sd a1,  8 * 11(sp)
            sd a2,  8 * 12(sp)
            sd a3,  8 * 13(sp)
            sd a4,  8 * 14(sp)
            sd a5,  8 * 15(sp)
            sd a6,  8 * 16(sp)
            sd a7,  8 * 17(sp)
            sd s0,  8 * 18(sp)
            sd s1,  8 * 19(sp)
            sd s2,  8 * 20(sp)
            sd s3,  8 * 21(sp)
            sd s4,  8 * 22(sp)
            sd s5,  8 * 23(sp)
            sd s6,  8 * 24(sp)
            sd s7,  8 * 25(sp)
            sd s8,  8 * 26(sp)
            sd s9,  8 * 27(sp)
            sd s10, 8 * 28(sp)
            sd s11, 8 * 29(sp)
            add t0, sp, 256
            sd t0,  8 * 30(sp)

            mv a0, sp
            call {handle_trap}

            ld ra,  8 * 0(sp)
            ld gp,  8 * 1(sp)
            ld tp,  8 * 2(sp)
            ld t0,  8 * 3(sp)
            ld t1,  8 * 4(sp)
            ld t2,  8 * 5(sp)
            ld t3,  8 * 6(sp)
            ld t4,  8 * 7(sp)
            ld t5,  8 * 8(sp)
            ld t6,  8 * 9(sp)
            ld a0,  8 * 10(sp)
            ld a1,  8 * 11(sp)
            ld a2,  8 * 12(sp)
            ld a3,  8 * 13(sp)
            ld a4,  8 * 14(sp)
            ld a5,  8 * 15(sp)
            ld a6,  8 * 16(sp)
            ld a7,  8 * 17(sp)
            ld s0,  8 * 18(sp)
            ld s1,  8 * 19(sp)
            ld s2,  8 * 20(sp)
            ld s3,  8 * 21(sp)
            ld s4,  8 * 22(sp)
            ld s5,  8 * 23(sp)
            ld s6,  8 * 24(sp)
            ld s7,  8 * 25(sp)
            ld s8,  8 * 26(sp)
            ld s9,  8 * 27(sp)
            ld s10, 8 * 28(sp)
            ld s11, 8 * 29(sp)
            ld sp,  8 * 30(sp)

            sret
        ",
        handle_trap = sym handle_trap
    );
}

extern "C" fn handle_trap(trap_frame: &TrapFrame) {
    let scause = scause::read();
    let stval = stval::read();
    let user_pc = sepc::read();
    match scause.cause() {
        scause::Trap::Interrupt(interrupt_vector) => {
            log::info!("interrupt: {interrupt_vector}");
            unsafe { sip::clear_pending(Interrupt::SupervisorTimer) };
            let time = time::read64();
            sbi::timer::set_timer(time + 10000000).unwrap();
        }
        scause::Trap::Exception(exception_code) => {
            panic!(
                "Exception: {exception_code} stval={stval:#X}. sepc={user_pc:#X} sp={:#X} trap frame={trap_frame:p}",
                trap_frame.sp
            );
        }
    }
    // panic!("unexpected trap. scause={scause:#X?}. stval={stval:#X}. sepc={user_pc:#X}.");
}
