/* Where your bootloader will load and start executing this kernel.
   Since you're building PIC/PIE (per our recent discussion), this
   is really just the *link-time* base for computing relative offsets
   — your bootloader adds its actual chosen load address on top. */
KERNEL_BASE = 0x0;

SECTIONS
{
    . = KERNEL_BASE;

    __kernel_start = .;

    .text : ALIGN(4K)
    {
        .text.first
        *(.text .text.*)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K)
    {
        *(.data .data.*)
    }

    .bss : ALIGN(4K)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /* A small fixed-size stack, placed right after everything else.
       Adjust size as needed — 8KB here as a starting point. */
    .stack (NOLOAD) : ALIGN(4K)
    {
        . += 8K;
        __stack_top = .;
    }

    __kernel_end = .;

    /* Discard sections you don't need in the final flat binary —
       debug info, unwind tables, and (if you're not consuming them
       yourself) the dynamic/relocation scaffolding from PIE linking. */
    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.note.*)
    }
}
