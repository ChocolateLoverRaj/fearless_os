ORG 0x9000

EFER equ 0xC0000080
LME equ 1 << 8

PE equ 1 << 0
PG equ 1 << 31

[BITS 32]
int_10_compat:
    ; Turn off paging
    mov eax, cr0
    and eax, ~PG
    mov cr0, eax

    ; Far jump to 16-bit protected mode
    jmp 0x18:int_10_protected_16

[BITS 16]
int_10_protected_16:
    ; Load data segment selectors with 16-bit indexes
    ; 0x30 is the offset in the GDT of the 16-bit data descriptor
    mov ax, 0x30
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; Load real mode IDT
    lidt [real_mode_idt]

    ; Disable protected mode
    mov eax, cr0
    and eax, ~PE
    mov cr0, eax

    ; Far jump to real mode
    jmp 0x0:int_10_real

[BITS 16]
int_10_real:
    ; Reload data segmetn registers with real mode values
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; We can re-enable interrupts now
    ; We should
    sti

    mov ax, di
    mov ah, 0x0E
    int 0x10

    ; Enable protection and paging in Cr0
    mov eax, cr0
    or eax, PE | PG
    mov cr0, eax

    ; Jump to 64-bit code
    jmp 0x8:int_10_ret

[BITS 64]
int_10_ret:
    cli
    ret

ALIGN 4
real_mode_idt:
.len:
    dw 0x03FF
.addr:
    dd 0x00000000
