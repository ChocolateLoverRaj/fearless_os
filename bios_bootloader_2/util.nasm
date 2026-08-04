[map all build/util.map]
ORG SELF_ADDR

EFER equ 0xC0000080
LME equ 1 << 8

PE equ 1 << 0
PG equ 1 << 31

; Clobbers ax
%macro ENTER_REAL 1
    cli

    push 0x10
    push %%compat
    retfq

[BITS 32]
%%compat:
    ; Disable paging
    mov eax, cr0
    and eax, ~PG
    mov cr0, eax

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

; Clobbers ax
%macro EXIT_REAL 1
    cli

    ; Enter protected mode
    mov eax, cr0
    or eax, PE | PG
    mov cr0, eax

    jmp 0x8:%1
%endmacro

table:
    dq int_10_long
    dq int_15_long
    dq extended_read_long

ALIGN 16
[BITS 64]
int_10_long:
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

ALIGN 16
[BITS 64]
int_15_long:
    push si
    ENTER_REAL int_15_real
[BITS 16]
int_15_real:
    pop es
    mov ebx, edx
    mov edx, 0x534D4150
    mov eax, 0xE820
    int 0x15
    ; outputs: eax, ebx, cl, carry flag
    ; Put eax and ebx onto the stack
    push ebx
    push eax
    ; put carry flag into dl
    setc dl
    mov dh, cl
    EXIT_REAL int_15_done
[BITS 64]
int_15_done:
    ; Put eax in lower 32 bits of rax and ebx in upper 32 bits
    pop rax
    ; Rdx will have the dl already
    ret

ALIGN 16
[BITS 64]
extended_read_long:
    push di
    ENTER_REAL extended_read_real
[BITS 16]
extended_read_real:
    pop ds
    mov ah, 0x42
    int 0x13
    setc al
    mov bx, ax
    EXIT_REAL extended_read_done
[BITS 64]
extended_read_done:
    mov ax, bx
    ret

ALIGN 4
real_mode_idt:
.len:
    dw 0x03FF
.addr:
    dd 0x00000000
