[map all build/util.map]
ORG SELF_ADDR

EFER equ 0xC0000080
LME equ 1 << 8

PE equ 1 << 0
PG equ 1 << 31

; Call this before pushing things onto the stack that you want real mode to access in its stack
%macro LOAD_REAL_ENV 0
    cmp word [table.stack_pointer], 0
    jz %%done
    xchg rsp, [table.stack_pointer]
%%done

    sub rsp, 10
    sgdt [rsp]
    lgdt [gdt_pointer]

    sub rsp, 10
    sidt [rsp]
%%done2
%endmacro

; Call this just before returning
%macro LOAD_LONG_ENV 0
    lidt [rsp]
    add rsp, 10

    lgdt [rsp]
    add rsp, 10

    cmp qword [table.stack_pointer], 0
    jz %%done
    xchg rsp, [table.stack_pointer]
%%done
%endmacro

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
.stack_pointer:
    dq 0
    dq int_10_long
    dq int_15_long
    dq extended_read_long
    dq vesa_get_controller_info_long
.dap_buffer:
    times 16 db 0
.int_15_buffer:
    times 24 db 0
.vbe_info_buffer:
    times 512 db 0

ALIGN 16
[BITS 64]
int_10_long:
    LOAD_REAL_ENV
    ENTER_REAL int_10_real
[BITS 16]
int_10_real:
    mov ax, di
    mov ah, 0x0E
    int 0x10
    EXIT_REAL int_10_done
[BITS 64]
int_10_done:
    LOAD_LONG_ENV
    ret

ALIGN 16
[BITS 64]
int_15_long:
    LOAD_REAL_ENV
    ENTER_REAL int_15_real
[BITS 16]
int_15_real:
    mov ebx, edi
    mov edx, 0x534D4150
    mov eax, 0xE820
    mov ecx, 24
    mov di, table.int_15_buffer
    int 0x15
    ; outputs: eax, ebx, cl, carry flag
    ; put carry flag into dl
    setc dl
    ; Put eax and ebx onto the stack
    push ebx
    push eax
    mov dh, cl
    EXIT_REAL int_15_done
[BITS 64]
int_15_done:
    ; Put eax in lower 32 bits of rax and ebx in upper 32 bits
    pop rax
    ; Rdx will have the dl already
    LOAD_LONG_ENV
    ret

ALIGN 16
[BITS 64]
extended_read_long:
    LOAD_REAL_ENV
    mov dx, di
    ENTER_REAL extended_read_real
[BITS 16]
extended_read_real:
    mov si, table.dap_buffer
    mov ah, 0x42
    int 0x13
    setc al
    mov bx, ax
    EXIT_REAL extended_read_done
[BITS 64]
extended_read_done:
    mov ax, bx
    LOAD_LONG_ENV
    ret

ALIGN 16
[BITS 64]
vesa_get_controller_info_long:
    LOAD_REAL_ENV
    ENTER_REAL vesa_get_controller_info_real
[BITS 16]
vesa_get_controller_info_real:
    mov ax, 0x4F00
    mov di, table.vbe_info_buffer
    int 0x10
    mov bx, ax
    EXIT_REAL vesa_get_controller_info_done
[BITS 64]
vesa_get_controller_info_done:
    LOAD_LONG_ENV
    mov ax, bx
    ret

ALIGN 4
real_mode_idt:
.len:
    dw 0x03FF
.addr:
    dd 0x00000000

ALIGN 8
gdt:
    .Null:
        dq 0x0000000000000000      ; 0x00: Null Descriptor
    .Code:
        dq 0x00209A0000000000      ; 0x08: 64-bit code descriptor
    .Code32:
        dq 0x00CF9A000000FFFF      ; 0x10: 32-bit code descriptor
    .Code16:
        dq 0x000F9A000000FFFF      ; 0x18: 16-bit code segment
    .Data:
        dq 0x0000920000000000      ; 0x20: 64-bit data descriptor
    .Data32:
        dq 0x00CF92000000FFFF      ; 0x28: 32-bit data descriptor
    .Data16:
        dq 0x000092000000FFFF
    .End:

gdt_pointer:
    .Size:
        ; Size of GDT - 1
        dw (gdt.End - gdt) - 1
    .Addr:
        ; Address of GDT
        dd gdt
