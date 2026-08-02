ORG 0x7C00
BITS 16

SELF_ADDR equ 0x7C00
;NEXT_STAGE_MEM_LEN equ <external>
;NEXT_STAGE_FILE_LEN equ <external>
;PAGE_TABLE_256T_ADDR equ <external>
;PAGE_TABLE_512G_ADDR equ <external>
;PAGE_TABLE_1G_ADDR equ <external>
;NEXT_STAGE_ADDR equ <external>
;NEXT_STAGE_JMP_ADDR equ <external>
KIB_NEEDED equ (NEXT_STAGE_ADDR + NEXT_STAGE_MEM_LEN + 0x400 - 1) / 0x400
SECTORS_TO_READ equ (NEXT_STAGE_FILE_LEN + 0x200 - 1) / 0x200

start:
    ; Disable interrupts while setting segmetn registers
    cli
    cld
    ; Some BIOSes don't set CS to 0, so we should for consistency
    jmp 0x0:after_reload_cs

after_reload_cs:
    xor si, si
    mov ss, si
    mov sp, start
    mov ds, si
    mov es, si
    ; It's okay for code after this to be interrupted
    sti

    ; Check for magic from MBR bootloader
    cmp eax, 0xA786B9FC
    jne error
    ; ebx = lower 32 bits of our partition's starting LBA
    ; ecx = upper 32 bits of our partition's starting LBA
    add [buffer.starting_lba], ebx
    adc [buffer.starting_lba + 0x4], ecx

    push ecx
    push ebx
    push dx

    ; Check if there is enough low memory
    int 0x12
    jc error
    cmp ax, KIB_NEEDED
    jl error

    ; Query A20 gate support
    mov ax, 0x2403
    int 0x15
    ; If carry, not support
    jc error
    ; If ah = 0, not supported
    test ah, ah
    jnz error
    ; Activate A20 gate
    mov ax, 0x2401
    int 0x15
    jc error
    test ah, ah
    jnz error

    ; Make sure the 0x42 extension exists
    mov ah, 0x41
    mov bx, 0x55AA
    int 0x13
    jc error
    test cx, 1 << 2
    jz error

read:
    ; ecx = sectors left to read
    mov ecx, SECTORS_TO_READ
.loop:
    test ecx, ecx
    jz .done
    mov eax, ecx
    ; now eax = sectors to read
    movzx esi, word [buffer.dest_offset]
    shr esi, 9
    mov ebx, 128
    sub ebx, esi
    ; now ebx = max sectors until we hit 64 KiB boundary
    cmp eax, ebx
    jbe .within_boundary
    mov eax, ebx
.within_boundary:
    ; At this point eax = sectors to read (max 128)
    cmp eax, 127
    jbe .small_enough
    mov eax, 127
.small_enough:
    ; At this point, eax = sectors to read (max 127)
    mov [buffer.transfer_count], al
    mov si, buffer
    push eax
    mov ah, 0x42
    int 0x13
    jc error
    pop eax

    ; Update sectors left to read
    sub ecx, eax

    ; Advance the starting LBA
    add [buffer.starting_lba], eax
    adc dword [buffer.starting_lba + 0x4], 0
    ; Advance the dest offset
    shl ax, 9
    add [buffer.dest_offset], ax
    ; Advance the dest segment if needed
    jnc .after_advance_segment
    add word [buffer.dest_segment], 0x1000
.after_advance_segment:
    jmp .loop
.done:

    ; Check whether long mode is supported or not
    ; Check whether CPUID is supported or not.
    ; It's supported, bit 0x200000 can be changed
    ; This original one will be preserved
    pushfd
    ; This one is just to pop immediately
    pushfd
    ; Toggle the bit
    xor dword [esp], 0x200000
    popfd
    ; Read it back and see if it's still there
    pushfd
    pop eax
    ; See which bits changed
    xor eax, [esp]
    ; Restore original eflags
    popfd
    ; If the bit changed, that means CPUID is supported
    test eax, 0x200000
    jz error
    ; CpuId is supported.
    ; Use this CPUID to check the highest CPUID function implemented
    mov eax, 0x80000000
    cpuid
    ; We need the function 0x80000001, so the highest must be at least this
    cmp eax, 0x80000001
    jb error
    ; Get feature flags
    mov eax, 0x80000001
    cpuid
    ; Bit 29 is long mode
    test edx, 1 << 29
    jz error

;    ; Find memory to put the next stage
;.loop:
;    mov di, INT_15_BUFFER_ADDR
;    mov bx, 0
;    mov edx, 0x534D4150
;    mov eax, 0xE820
;    mov ecx, 24
;    int 0x15
;    jc error
;    cmp eax, 0x534D4150
;    jne error
;    test bx, bx
;    jz error
;    cmp [INT_15_BUFFER_ADDR + 0x8], NEXT_STAGE_LEN
;    jge .found_mem
;    cmp [INT_15_BUFFER_ADDR + 0xC], 0
;   jz .loop
;.found_mem:

    ; Load the GDT and IDT, located in stage_1.asm, and part of the first sector
    lgdt [gdt_pointer]
    lidt [idt]

    ; Create page tables, identity-mapping the bottom 1 GiB
    PRESENT equ 1 << 0
    WRITABLE equ 1 << 1
    USER_ACCESSIBLE equ 1 << 2
    WRITE_THROUGH equ 1 << 3
    NO_CACHE equ 1 << 4
    PAGE_SIZE equ 1 << 7

    ; Create the top level page table
    ; Create the first entry
    ; Point to 8 KiB address
    mov di, PAGE_TABLE_256T_ADDR
    mov eax, PRESENT | WRITABLE | ((PAGE_TABLE_512G_ADDR >> 12) << 12)
    stosd
    ; Zero the rest
    xor ax, ax
    mov cx, 0x7FE
    rep stosw

    ; Create the next level page table
    ; di = 0x6000 already
    mov eax, PRESENT | WRITABLE | ((PAGE_TABLE_1G_ADDR >> 12) << 12)
    stosd
    ; Zero the rest
    xor ax, ax
    mov cx, 0x7FE
    rep stosw

create_page_table_1g:
    ; Create the next level page table with entries mapping 2 MiB pages
    ; di already at target
    ; mov di, PAGE_TABLE_1G_ADDR
    mov cx, 512
    xor ebx, ebx
.loop:
    mov eax, ebx
    or eax, PRESENT | WRITABLE | PAGE_SIZE
    stosd
    xor eax, eax
    stosd
    add ebx, 1 << 21
    loop .loop


    ; We don't have a valid IDT for handling interrupts in long mode
    cli

    ; Enable Cr4 flags for long mode
    PAE equ 1 << 5
    PGE equ 1 << 7

    mov eax, PAE | PGE
    mov cr4, eax

    ; Set Cr3 to point to our page table
    mov eax, PAGE_TABLE_256T_ADDR
    mov cr3, eax

    ; Enable LME in EFER
    EFER equ 0xC0000080
    LME equ 1 << 8

    mov ecx, EFER
    rdmsr
    or eax, LME
    wrmsr

    ; Enable protection and paging in Cr0
    PE equ 1 << 0
    PG equ 1 << 31

    mov eax, cr0
    or eax, PE | PG
    mov cr0, eax

    ; Load CS with 64 bit segment and flush the instruction cache
    jmp 0x8:long_mode

error:
    jmp $

; Technically the spec says we can read into a 64-bit address and read a 32-bit sector count, but QEMU doesn't support it
ALIGN 2
buffer:
    .len:
        ; Buffer len
        db buffer.end - buffer
    .reserved_0:
        ; Reserved, must be 0
        db 0
    .transfer_count:
        ; # of blocks to transfer
        db (NEXT_STAGE_FILE_LEN + 512 - 1) / 512
    .reserved_1:
        ; Reserved, must be 0
        db 0
    .dest_offset:
        ; Destination offset
        dw NEXT_STAGE_ADDR
    .dest_segment:
        ; Destination segment
        dw 0
    .starting_lba:
        ; Starting LBA (64 bits)
        dq 1
    .end:


ALIGN 8
gdt:
.null:
    dq 0x0000000000000000      ; 0x00: Null Descriptor
.code64:
    dq 0x00209A0000000000
.data64:
    dq 0x0000920000000000
.end:

ALIGN 4
idt:
.len:
    dw 0
.addr:
    dd 0

gdt_pointer:
; Size of GDT - 1
.size:
    dw (gdt.end - gdt) - 1
; Address of GDT
.addr:
    dd gdt

[BITS 64]
long_mode:
    pop dx
    pop rsi
    jmp NEXT_STAGE_JMP_ADDR
