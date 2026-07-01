use core::{arch::naked_asm, ptr::NonNull};

use aarch64_cpu::registers::{ELR_EL1, ESR_EL1, Readable, VBAR_EL1, Writeable};
use log::info;

use crate::{aarch64::logger::uart, bcm2835_aux::Bcm2835Aux, bcm2836_armctrl_ic::Bcm2836ArmCtrlIc};

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

/// # Safety
/// Modifies VBAR_EL1
pub unsafe fn init() {
    VBAR_EL1.set(vector_table as *const () as u64);
}
