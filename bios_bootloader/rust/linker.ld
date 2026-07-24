SECTIONS
{
    . = LOAD_ADDR;
    __kernel_start = .;

    .text :
    {
        .text.start
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

    .bss : ALIGN(8)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(8);
        __bss_end = .;
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

__bss_u64s_to_copy = (__bss_end - __bss_start) / 8;
