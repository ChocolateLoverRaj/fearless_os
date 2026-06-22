#![cfg_attr(target_arch = "arm", feature(stdarch_arm_neon_intrinsics))]
mod logger;
mod panic_handler;

use core::{
    arch::naked_asm,
    ptr::{NonNull, addr_of},
};

use aarch64_cpu::{
    asm::wfi,
    registers::{
        CurrentEL, DAIF, ELR_EL1, ELR_EL2, ESR_EL1, HCR_EL2, MPIDR_EL1, Readable, SP_EL1, SPSR_EL2,
        VBAR_EL1, Writeable,
    },
};
use log::info;

use self::{bcm2835_aux::Bcm2835Aux, bcm2836_armctrl_ic::Bcm2836ArmCtrlIc, logger::uart};

unsafe extern "C" {
    static __kernel_start: u8;
    static __stack_top: u8;
}

#[unsafe(link_section = ".text._header")]
// Prevent this function from being removed
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn _start() {
    naked_asm!(
        "
        // Get the actual start address (which is also the load offset)
        adr x1, _start
        b {start}
        ",
        start = sym start
    )
}

#[unsafe(naked)]
pub unsafe extern "C" fn start() {
    naked_asm!(
        "
        // Do relocations
        // Get the actual start and end addresses
        ldr x2, =__rel_start
        add x2, x2, x1
        ldr x3, =__rel_end
        add x3, x3, x1
        // Loop through all relocations
        .reloc_loop:
            // Exit if we're done
            cmp x2, x3
            beq .reloc_loop_done

            // Load the relocation type to make sure it is R_AARCH64_RELATIV
            ldr x4, [x2, #8]
            // The type is the lower 32 bits
            cmp w4, #0x403
            bne .unknown_reloc

            // Read `r_addend`
            ldr x4, [x2, #16]
            // Add the offset
            add x4, x4, x1

            // Read the location to be patched
            ldr x5, [x2]
            // Add the offset
            add x5, x5, x1

            // Patch the location
            str x4, [x5]

            // Go to the next relocation
            add x2, x2, #24
            b .reloc_loop

        .unknown_reloc:
            b .unknown_reloc

        .reloc_loop_done:

        // Set stack pointer
        ldr x2, =__stack_top
        add x2, x2, x1
        mov sp, x2

        // Clear bss
        ldr x2, =__bss_start
        add x2, x2, x1
        ldr x3, =__bss_end
        add x3, x3, x1

        .zero_bss_loop:
            // Exit the loop if we're done
            cmp x2, x3
            beq .zero_bss_loop_done

            // Write *x2 = 0_u64; x2 += 8;
            str     xzr, [x2], #8

            b .zero_bss_loop

        .zero_bss_loop_done:
            bl      {kernel_main}
        ",
        kernel_main = sym kernel_main
    )
}

unsafe extern "C" fn kernel_main(fdt_addr: usize) -> ! {
    unsafe { logger::init() };

    let el = CurrentEL
        .read_as_enum::<CurrentEL::EL::Value>(CurrentEL::EL)
        .unwrap();
    let kernel_addr = addr_of!(__kernel_start).addr();
    let mpidr = MPIDR_EL1.get();
    info!(
        "Hello from the kernel! Loaded at {kernel_addr:#X}, FDT Addr: {fdt_addr:#X}, EL: {el:?}, mpidr: {mpidr:#X}"
    );

    // The Linux boot protocol says that the kernel is started in either EL2 or EL1
    // Since we're not doing hypervisor stuff, drop down to EL1 if we're in EL2
    if el == CurrentEL::EL::Value::EL2 {
        // Set EL1 execution state to AArch64.
        HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);

        // Configure that eret will go into EL1 with interrupts disabled
        SPSR_EL2.write(
            SPSR_EL2::D::Masked
                + SPSR_EL2::A::Masked
                + SPSR_EL2::I::Masked
                + SPSR_EL2::F::Masked
                + SPSR_EL2::M::EL1h,
        );

        // Configure to jump to this fn on eret
        ELR_EL2.set(kernel_main_el1 as *const () as u64);

        // Configure the stack pointer on eret
        let stack_top = addr_of!(__stack_top).addr();
        SP_EL1.set(stack_top as u64);

        info!("Dropping to EL1");
        unsafe {
            core::arch::asm!("eret", in("x0") fdt_addr, options(nomem, nostack, noreturn));
        }
    } else {
        kernel_main_el1(fdt_addr);
    }

    // loop {
    //     let byte = uart.read_sync();
    //     uart.write_sync_no_flush(byte.to_ascii_uppercase());
    // }
}

extern "C" fn kernel_main_el1(fdt_addr: usize) -> ! {
    let el = CurrentEL
        .read_as_enum::<CurrentEL::EL::Value>(CurrentEL::EL)
        .unwrap();
    assert_eq!(el, CurrentEL::EL::Value::EL1);
    let daif = DAIF.get();
    info!("Hello from EL1. DAIF: {daif:#X}");

    // Configure interrupts
    VBAR_EL1.set(vector_table as *const () as u64);

    let rx_enabled = uart().try_lock().unwrap().rx_interrupt_enabled();
    info!("RX int enabled: {rx_enabled}");
    let tx_enabled = uart().try_lock().unwrap().tx_interrupt_enabled();
    info!("TX int enabled: {tx_enabled}");

    uart().try_lock().unwrap().set_rx_interrupt_enabled(true);
    uart().try_lock().unwrap().set_tx_interrupt_enabled(false);

    let rx_enabled = uart().try_lock().unwrap().rx_interrupt_enabled();
    info!("RX int enabled: {rx_enabled}");
    let tx_enabled = uart().try_lock().unwrap().tx_interrupt_enabled();
    info!("TX int enabled: {tx_enabled}");

    let mut ic = unsafe { Bcm2836ArmCtrlIc::new(NonNull::new(0x3F00_B200 as *mut _).unwrap()) };
    let aux_int_en = ic.get_interrupt_enabled(29);
    info!("AUX int enabled: {aux_int_en}");
    ic.set_interrupt_enabled(29, true);
    let aux_int_en = ic.get_interrupt_enabled(29);
    info!("AUX int enabled: {aux_int_en}");

    let aux_int_pending = ic.is_pending(29);
    info!("AUX int pending: {aux_int_pending}");
    let rx_int_pending = uart().try_lock().unwrap().rx_interrupt_pending();
    info!("RX int pending: {rx_int_pending}");
    let tx_int_pending = uart().try_lock().unwrap().tx_interrupt_pending();
    info!("TX int pending: {tx_int_pending}");

    // Enable interrupts
    DAIF.write(DAIF::D::CLEAR + DAIF::A::CLEAR + DAIF::I::CLEAR + DAIF::F::CLEAR);
    loop {
        wfi();
    }
}

#[unsafe(naked)]
extern "C" fn vector_table() {
    naked_asm!(
        "
        .macro SAVE_CONTEXT
            // Make room for 22 registers (x0-x18, x29, x30, +1 for alignment)
            sub     sp, sp, #176

            // Save volatile registers in pairs
            stp     x0,  x1,  [sp, #16 * 0]
            stp     x2,  x3,  [sp, #16 * 1]
            stp     x4,  x5,  [sp, #16 * 2]
            stp     x6,  x7,  [sp, #16 * 3]
            stp     x8,  x9,  [sp, #16 * 4]
            stp     x10, x11, [sp, #16 * 5]
            stp     x12, x13, [sp, #16 * 6]
            stp     x14, x15, [sp, #16 * 7]
            stp     x16, x17, [sp, #16 * 8]

            // Save x18 and x29 (Frame Pointer)
            stp     x18, x29, [sp, #16 * 9]

            // Save x30 (Link Register)
            str     x30, [sp, #16 * 10]
        .endm
        .macro RESTORE_CONTEXT
            // Restore x30 (Link Register)
            ldr     x30, [sp, #16 * 10]

            // Restore x18 and x29
            ldp     x18, x29, [sp, #16 * 9]

            // Restore x0 through x17
            ldp     x16, x17, [sp, #16 * 8]
            ldp     x14, x15, [sp, #16 * 7]
            ldp     x12, x13, [sp, #16 * 6]
            ldp     x10, x11, [sp, #16 * 5]
            ldp     x8,  x9,  [sp, #16 * 4]
            ldp     x6,  x7,  [sp, #16 * 3]
            ldp     x4,  x5,  [sp, #16 * 2]
            ldp     x2,  x3,  [sp, #16 * 1]
            ldp     x0,  x1,  [sp, #16 * 0]

            // Shrink stack
            add     sp, sp, #176

            eret
        .endm

        // The table must be aligned
        .balign 2048
        // SP_EL0
        // Synchronous Exception
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #0
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // IRQ
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #1
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // FIQ
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #2
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // Asynchronous Exception
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #3
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // SP_ELx
        // Syncronous Exception
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #4
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // IRQ
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #5
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // FIQ
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #6
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // Asynchronos Exception
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #7
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // From lower EL
        // Synchronous Exception
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #8
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // IRQ
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #9
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // FIQ
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #10
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // Asynchronous Exception
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #11
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // From lower EL
        // Synchronous Exception
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #12
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // IRQ
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #13
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // FIQ
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #14
            bl {interrupt_handler}
            RESTORE_CONTEXT
        // FIQ
        .balign 0x80
            SAVE_CONTEXT
            mov x0, #15
            bl {interrupt_handler}
            RESTORE_CONTEXT
        ",
        interrupt_handler = sym interrupt_handler,
    )
}

unsafe extern "C" fn interrupt_handler(source: usize) {
    let esr = ESR_EL1.get();
    let elr = ELR_EL1.get();
    info!("interrupt / exception. source: {source}. esr: {esr:#X}. elr: {elr:#X}.");

    if source == 5 {
        let ic = unsafe { Bcm2836ArmCtrlIc::new(NonNull::new(0x3F00_B200 as *mut _).unwrap()) };

        let aux_int_pending = ic.is_pending(29);
        info!("AUX int pending: {aux_int_pending}");

        let aux = unsafe { Bcm2835Aux::new(NonNull::new(0x3F215000 as *mut _).unwrap()) };
        let mini_uart_int_pending = aux.mini_uart_irq_pending();
        info!("Mini UART int pending: {mini_uart_int_pending}");

        let uart_int_pending = uart().try_lock().unwrap().interrupt_pending();
        info!("UART int pending: {uart_int_pending}");
        let rx_int_pending = uart().try_lock().unwrap().rx_interrupt_pending();
        info!("RX int pending: {rx_int_pending}");
        let tx_int_pending = uart().try_lock().unwrap().tx_interrupt_pending();
        info!("TX int pending: {tx_int_pending}");

        if rx_int_pending {
            let mut read_count = 0;
            while let Some(_byte) = uart().try_lock().unwrap().try_read() {
                read_count += 1;
            }
            if read_count > 0 {
                info!("Read RX bytes: {read_count}");
            }
        }
    }
}
