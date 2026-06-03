#![no_std]
#![no_main]

use core::arch::naked_asm;

use riscv::{
    asm::wfi,
    interrupt::Interrupt,
    register::{
        scause, sepc,
        sie::{self, Sie},
        sip,
        sstatus::{self, Sstatus},
        stval,
        stvec::{self, Stvec, TrapMode},
        time,
    },
};
use sbi::hsm::SuspendType;

// These variables are defined in the linker script
unsafe extern "C" {
    static __kernel_start: usize;
    static __kernel_end: usize;
    static __bss_start: usize;
    static __bss_end: usize;
    static __stack_top: usize;
}

/// OpenSBI passes the HART ID in the `a0` register and a pointer to the device tree in the `a1`
/// register. Since we don't modify those registers, we can just jump to `kernel_main` and those
/// two inputs will be passed to it.
#[unsafe(link_section = ".text._header")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
extern "C" fn _start() {
    naked_asm!(
        "
            j {start}
        ",
        start = sym start
    )
}
#[unsafe(naked)]
extern "C" fn start() {
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

extern "C" fn kernel_main(hart_id: usize, fdt_addr: usize) -> ! {
    unsafe {
        stvec::write(Stvec::new(
            kernel_entry as *const () as usize,
            TrapMode::Direct,
        ))
    };
    let mut sstatus = Sstatus::from_bits(0);
    sstatus.set_sie(true);
    unsafe { sstatus::write(sstatus) };

    let time = time::read64();
    sbi::timer::set_timer(time + 10000).unwrap();

    let msg = "Hello World!\r\n";
    for &byte in msg.as_bytes() {
        sbi::legacy::console_putchar(byte);
    }

    let mut sie = Sie::from_bits(0);
    sie.set_stimer(true);
    unsafe { sie::write(sie) };

    loop {
        sbi::legacy::console_putchar(b'.');
        wfi();
    }
    // sbi::legacy::shutdown()
}

#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[repr(C, packed)]
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
            csrw sscratch, sp
            addi sp, sp, -4 * 31

            sd ra,  4 * 0(sp)
            sd gp,  4 * 1(sp)
            sd tp,  4 * 2(sp)
            sd t0,  4 * 3(sp)
            sd t1,  4 * 4(sp)
            sd t2,  4 * 5(sp)
            sd t3,  4 * 6(sp)
            sd t4,  4 * 7(sp)
            sd t5,  4 * 8(sp)
            sd t6,  4 * 9(sp)
            sd a0,  4 * 10(sp)
            sd a1,  4 * 11(sp)
            sd a2,  4 * 12(sp)
            sd a3,  4 * 13(sp)
            sd a4,  4 * 14(sp)
            sd a5,  4 * 15(sp)
            sd a6,  4 * 16(sp)
            sd a7,  4 * 17(sp)
            sd s0,  4 * 18(sp)
            sd s1,  4 * 19(sp)
            sd s2,  4 * 20(sp)
            sd s3,  4 * 21(sp)
            sd s4,  4 * 22(sp)
            sd s5,  4 * 23(sp)
            sd s6,  4 * 24(sp)
            sd s7,  4 * 25(sp)
            sd s8,  4 * 26(sp)
            sd s9,  4 * 27(sp)
            sd s10, 4 * 28(sp)
            sd s11, 4 * 29(sp)

            csrr a0, sscratch
            sd a0, 4 * 30(sp)

            mv a0, sp
            call {handle_trap}

            ld ra,  4 * 0(sp)
            ld gp,  4 * 1(sp)
            ld tp,  4 * 2(sp)
            ld t0,  4 * 3(sp)
            ld t1,  4 * 4(sp)
            ld t2,  4 * 5(sp)
            ld t3,  4 * 6(sp)
            ld t4,  4 * 7(sp)
            ld t5,  4 * 8(sp)
            ld t6,  4 * 9(sp)
            ld a0,  4 * 10(sp)
            ld a1,  4 * 11(sp)
            ld a2,  4 * 12(sp)
            ld a3,  4 * 13(sp)
            ld a4,  4 * 14(sp)
            ld a5,  4 * 15(sp)
            ld a6,  4 * 16(sp)
            ld a7,  4 * 17(sp)
            ld s0,  4 * 18(sp)
            ld s1,  4 * 19(sp)
            ld s2,  4 * 20(sp)
            ld s3,  4 * 21(sp)
            ld s4,  4 * 22(sp)
            ld s5,  4 * 23(sp)
            ld s6,  4 * 24(sp)
            ld s7,  4 * 25(sp)
            ld s8,  4 * 26(sp)
            ld s9,  4 * 27(sp)
            ld s10, 4 * 28(sp)
            ld s11, 4 * 29(sp)

            ld sp,  4 * 30(sp)
            sret
        ",
        handle_trap = sym handle_trap
    );
}

extern "C" fn handle_trap(_trap_frame: &TrapFrame) {
    let scause = scause::read();
    let stval = stval::read();
    let user_pc = sepc::read();
    let msg = "Timer Interrupt!\r\n";
    for &byte in msg.as_bytes() {
        sbi::legacy::console_putchar(byte);
    }
    unsafe { sip::clear_pending(Interrupt::SupervisorTimer) };
    let time = time::read64();
    sbi::timer::set_timer(time + 10000000).unwrap();
    // panic!("unexpected trap. scause={scause:#X?}. stval={stval:#X}. sepc={user_pc:#X}.");
}
