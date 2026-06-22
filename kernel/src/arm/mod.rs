mod logger;
mod panic_handler;

use core::{arch::naked_asm, ptr::NonNull};

use atags::Atags;
use fdt::Fdt;
use log::info;

#[unsafe(link_section = ".text._header")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
extern "C" fn _start() {
    naked_asm!(
        "
        // Get the addres where the start of our kernel was loaded
        sub r3, pc, #8
        // Jump to the entry assembly because the header section isn't big enough to old it
        b {entry}
        ",
        entry = sym entry
    )
}

#[unsafe(naked)]
unsafe extern "C" fn entry() -> ! {
    naked_asm!(
        "
        // Read the Multiprocessor Affinity Register (MPIDR)
        mrc p15, 0, r4, c0, c0, 5
        // Read the implementer
        lsr r5, r4, #24
        and r5, r5, #0xff
        // Check if it's Broadcom
        cmp r5, #0x42
        bne .reloc
        // Read the part number
        lsr r5, r4, #4
        ldr r6, =0xC07
        cmp r5, r6
        bne .reloc
        // At this point it's a Raspberry Pi 2
        // Shut off extra cores
        // Read the core id
        and r5, r4, #3
        // Halt loop if the core id is not 0
        cmp r5, #0
        bne halt

        .reloc:
        // Apply relocations
        // Get the actual ptr to start of relocations
        ldr r4, =__rel_start
        add r4, r4, r3
        // Get the actual ptr to end of relocations
        ldr r5, =__rel_end
        add r5, r5, r3
        // Get the difference in (actual address we're loaded in - the initial address relocations are based on)
        // Since our default ELF start address is 0x0, the diference is (r3 - 0 = r3), so we can just use r3

        // Loop through all relocations. Each relocation is a (u32, u32).
        // The first u32 is an offset from the start of the kernel binary file to the pointer in memory we need to update
        // The second u32 contains the relocation type in the lower byte
        .reloc_loop:
            // Exit the loop once we're done
            cmp r4, r5
            beq .reloc_done

            // Make sure the relocation is R_ARM_RELATIVE
            // So far this is the only type of relocation our binary has
            // But we can handle other types if they show up
            // Load the second u32
            ldr r7, [r4, #4]
            // We want to know the type which is stored in the lower 8 bits
            and r7, r7, #0xff
            cmp r7, #0x17
            bne .unknown_reloc

            // Load the first u32
            ldr r7, [r4]
            // Get the real address of that location in our memory by adjusting by the offset
            add r7, r7, r3
            // Load the pointer that needs to be adjusted
            ldr r8, [r7]
            // Adjust the pointer
            add r8, r8, r3
            // Store the updated value
            str r8, [r7]

            // Go to the next relocation
            add r4, r4, #8
            b .reloc_loop

        .unknown_reloc:
            b .unknown_reloc

        .reloc_done:

        // Set the stack pointer to the stack space we reserved in the linker script
        ldr sp, =__stack_top
        add sp, sp, r3

        // Zero the BSS. Zero it by 4 * usize at a time instead of one byte or one usize at a time
        ldr r4, =__bss_start
        add r4, r4, r3
        ldr r9, =__bss_end
        add r9, r9, r3
        // Set r5-r8 to 0
        mov r5, #0
        mov r6, #0
        mov r7, #0
        mov r8, #0
        // Start by checking for the end condition
        b while

        do:
            // This stores the values of registers r5-r8 at the value of r4, incrementing r4 by
            // size_of::<usize> as it stores each register
            stmia r4!, {{r5-r8}}

        while:
            // If r4 < r9, jump to `do`
            cmp r4, r9
            blo do
            // Else, continue executing the instructions below
            // Call kernel_main
            blx {kernel_main}

        halt:
            wfe
            b halt
        ",
        kernel_main = sym main
    )
}

unsafe extern "C" fn main(_r0: usize, _machine_id: usize, atags_or_fdt_ptr: usize) {
    unsafe { logger::init() };
    match unsafe { Fdt::from_ptr(atags_or_fdt_ptr as *mut _) } {
        Ok(fdt) => {
            info!("Got FDT");
        }
        Err(fdt::FdtError::BadMagic) => {
            let atags = unsafe { Atags::new(NonNull::new(atags_or_fdt_ptr as *mut _).unwrap()) };
            info!("Got ATAGs");
            for tag in atags.iter() {
                info!("ATAG: {:?}", tag);
            }
        }
        Err(e) => {
            panic!("Error parsing FDT: {e:?}");
        }
    }
    // const UART0_DR: *mut u32 = 0x3F20_1000 as *mut u32;
    // const UART0_FR: *const u32 = (0x3F20_1000 + 0x18) as *const u32;
    // const UART_FR_TXFF: u32 = 1 << 5;
    // let message = b"Hello World\n";

    // for &byte in message.iter() {
    //     unsafe {
    //         // Wait until the Transmit FIFO is not full
    //         while (core::ptr::read_volatile(UART0_FR) & UART_FR_TXFF) != 0 {
    //             core::arch::asm!("nop", options(nomem, nostack, preserves_flags));
    //         }

    //         // Handle carriage return for standard serial terminals
    //         if byte == b'\n' {
    //             core::ptr::write_volatile(UART0_DR, b'\r' as u32);
    //             while (core::ptr::read_volatile(UART0_FR) & UART_FR_TXFF) != 0 {
    //                 core::arch::asm!("nop", options(nomem, nostack, preserves_flags));
    //             }
    //         }

    //         // Write the byte to the Data Register
    //         core::ptr::write_volatile(UART0_DR, byte as u32);
    //     }
    // }

    info!("Hello");
    // if let Some(stdout) = fdt.chosen().stdout() {
    //     if let Some(compatible) = stdout.compatible() {}
    // }
}
