#![no_std]
#![no_main]
// #![feature(stdarch_arm_hints)]
#![cfg_attr(target_arch = "arm", feature(stdarch_arm_neon_intrinsics))]
mod bcm2835_aux_uart;
mod logger;
mod panic_handler;

use core::{arch::naked_asm, ptr::addr_of};

use aarch64_cpu::{
    asm::wfi,
    registers::{CurrentEL, ELR_EL2, HCR_EL2, Readable, SP_EL1, SPSR_EL2, Writeable},
};
use log::info;

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
    info!("Hello from the kernel! Loaded at {kernel_addr:#X}, FDT Addr: {fdt_addr:#X}, EL: {el:?}");

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

    loop {
        wfi();
    }

    // loop {
    //     let byte = uart.read_sync();
    //     uart.write_sync_no_flush(byte.to_ascii_uppercase());
    // }
}

extern "C" fn kernel_main_el1(fdt_addr: usize) {
    let el = CurrentEL
        .read_as_enum::<CurrentEL::EL::Value>(CurrentEL::EL)
        .unwrap();
    assert_eq!(el, CurrentEL::EL::Value::EL1);
    info!("Hello from EL1");
}
