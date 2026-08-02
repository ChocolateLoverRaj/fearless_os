[map all build/util.map]
ORG 0x9000

EFER equ 0xC0000080
LME equ 1 << 8

PE equ 1 << 0
PG equ 1 << 31

%macro ENTER_REAL 1
    cli

    ; Disable paging
    mov eax, cr0
    and eax, ~PG
    mov cr0, eax

[BITS 32]
    jmp 0x18:%%protected_16

[BITS 16]
%%protected_16:
    ; 16-bit protected-mode data selector
    mov ax, 0x30
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Real-mode IVT
    lidt [real_mode_idt]

    ; Leave protected mode
    mov eax, cr0
    and eax, ~PE
    mov cr0, eax

    ; Reload real-mode segments
    jmp 0:%%real

%%real:
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    sti
    jmp %1
%endmacro


%macro EXIT_REAL 1
    cli

    ; Enter protected mode
    mov eax, cr0
    or eax, PE | PG
    mov cr0, eax

    jmp 0x8:%1
%endmacro

[BITS 32]
int_10_compat:
    ENTER_REAL int_10_real
[BITS 16]
int_10_real:
    mov ax, di
    mov ah, 0x0E
    int 0x10
    EXIT_REAL int_10_done
[BITS 64]
int_10_done:
    ret

ALIGN 4
real_mode_idt:
.len:
    dw 0x03FF
.addr:
    dd 0x00000000
