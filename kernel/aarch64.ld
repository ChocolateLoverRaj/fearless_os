SECTIONS
{
    __kernel_start = .;

    .text :
    {
        /* We are following the Linux 64-bit ARM boot protocol.
           https://docs.kernel.org/arch/arm64/booting.html
        */

        /* The kernel is jumped to the first byte of the zImage. The first 36 bytes are for instructions */
        KEEP(*(.text._header));
        ASSERT(. <= __kernel_start + 0x8, "header instructions don't fit");

        /* At 0x2, there is `text_offset`. We'll leave this at 0x0 */
        . = __kernel_start + 0x8;
        QUAD(0x0);
        /* `image_size` */
        QUAD(__kernel_end - __kernel_start);
        /* `flags` */
        QUAD(0x0);
        /* `res2` */
        QUAD(0x0);
        /* `res3` */
        QUAD(0x0);
        /* `res4` */
        QUAD(0x0);
        /* MAGIC */
        LONG(0x644d5241);
        /* `res5` */
        LONG(0x0);

        /* After that we can have whatever we want */
        *(.text .text.*);
    }

    /* Global immutable vars */
    .rodata :
    {
        *(.rodata .rodata.*)
    }

    /* Global mutable vars */
    .data :
    {
        *(.data)
    }

    /* Desribes how to do relocations, not needed after relocations are done */
    .rela.dyn : {
        /* Must be 8-byte aligned so we can do aligned u64 access */
        . = ALIGN(8);
        __rel_start = . - __kernel_start;
        *(.rela*)
        __rel_end = . - __kernel_start;
    }

    /* This is the end of the .bin file after objcopying */

    /* This is where global variables that are initially 0 are */
    .bss :
    {
        /* Make it aligned to 8 bytes so we can use u64 writes to zero it */
        . = ALIGN(8);
        __bss_start = . - __kernel_start;
        *(.bss, .bss.*)
        . = ALIGN(8);
        __bss_end = . - __kernel_start;
    }

    /* Stack must be 16 bytes aligned */
    . = ALIGN(16);
    . += 8 * 1024;
    __stack_top = . - __kernel_start;

    __kernel_end = .;
}
