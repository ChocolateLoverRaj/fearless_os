; STAGE_2_ADDR will be externally supplied
; PAGE_TABLES_ADDR will be externally supplied
ORG STAGE_2_ADDR
BITS 16

Start:
        ; Check whether we support Long Mode or not.
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
        jz NoCpuId

        ; CpuId is supported.
        ; Use this CPUID to check the highest CPUID function implemented
        mov eax, 0x80000000
        cpuid
        ; We need the function 0x80000001, so the highest must be at least this
        cmp eax, 0x80000001
        jb NoExtendedFunction
        ; Get feature flags
        mov eax, 0x80000001
        cpuid
        ; Bit 29 is long mode
        test edx, 1 << 29
        jz NoLongMode

EnableA20:
        mov     ax, 0x2403           ; Query A20 gate support
        int     0x15
        jc      Int15NotSupported        ; INT 0x15 is not supported
        test    ah, ah
        jnz     Int15NotSupported        ; INT 0x15 is not supported

        mov     ax, 0x2402           ; Get A20 gate status
        int     0x15
        jc      Int15Failed              ; Couldn't get status
        test    ah, ah
        jnz     Int15Failed              ; Couldn't get status
        test    al, al
        jnz     .Enabled           ; AL = 1, A20 gate is already activated

        mov     ax, 0x2401           ; Activate A20 gate
        int     0x15
        jc      ActivateA20Failed              ; Couldn't activate the gate
        test    ah, ah
        jnz     ActivateA20Failed              ; Couldn't activate the gate

    .Enabled:

GetMemory:
        ; Load the GDT and IDT, located in stage_1.asm, and part of the first sector
        lgdt [GdtPointer]
        lidt [Idt]

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
        mov di, PAGE_TABLES_ADDR
        mov eax, PRESENT | WRITABLE | (((PAGE_TABLES_ADDR + 0x1000) >> 12) << 12)
        stosd
        ; Zero the rest
        xor ax, ax
        mov cx, 0x7FE
        rep stosw

        ; Create the next level page table
        mov eax, PRESENT | WRITABLE | (((PAGE_TABLES_ADDR + 0x2000) >> 12) << 12)
        stosd
        ; Zero the rest
        xor ax, ax
        mov cx, 0x7FE
        rep stosw

        ; Create the next level page table with entries mapping 2 MiB pages
        mov cx, 512
        xor ebx, ebx
    .Loop:
        mov eax, ebx
        or eax, PRESENT | WRITABLE | PAGE_SIZE
        stosd
        xor eax, eax
        stosd
        add ebx, 1 << 21
        loop .Loop

        ; Enable Cr4 flags for long mode
        PAE equ 1 << 5
        PGE equ 1 << 7

        mov eax, PAE | PGE
        mov cr4, eax

        ; Set Cr3 to point to our page table
        mov eax, PAGE_TABLES_ADDR
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
        jmp 0x8:LongMode

NoCpuId:
        jmp $

NoExtendedFunction:
        jmp $

NoLongMode:
        jmp $

Int15NotSupported:
        jmp $

Int15Failed:
        jmp $

ActivateA20Failed:
        jmp $

ErrorGettingMemory:
        jmp $

NotEnoughMem:
        jmp $

ALIGN 8
Gdt:
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

ALIGN 4
Idt:
    .Length:
        dw 0
    .Addr:
        dd 0

GdtPointer:
    .Size:
        ; Size of GDT - 1
        dw (Gdt.End - Gdt) - 1
    .Addr:
        ; Address of GDT
        dd Gdt

ALIGN 4
RealModeIdt:
    .Length:
        dw 0x03FF
    .Addr:
        dd 0x00000000

PrintReal:
        ; 5. Load data segment selectors with 16-bit indexes:
        ; The offset of your .Data16 descriptor in the GDT is 0x30
        mov ax, 0x30
        ; Load all data and stack segments with the same 16-bit data selector
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
        mov ss, ax

        ; 6. Load real mode IDT:
        lidt [RealModeIdt]

        ; 7. Disable protected mode:
        mov eax, cr0
        and eax, ~PE
        mov cr0, eax

        ; 8. Far jump to real mode:
        jmp 0x0:.Real

    .Real
        ; 9. Reload data segment registers with real mode values:
        xor ax, ax
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
        mov ss, ax

        ; 10. Set stack pointer to appropriate value:
        ; We're skipping this since it should be intact

        ; Enable interrupts:
        ; We're skipping this, it might be needed though

        mov ax, di
        mov ah, 0x0E
        int 0x10
        mov ax, di
        mov ah, 0x0E
        int 0x10
        jmp $



[BITS 64]
ALIGN 16
Print:
        ; Enter 32 bit compatibility mode by long jumping
        ; This is how u long jump in real mode:
        ; CS
        push 0x10
        ; jmp address
        push PrintCompat
        retfq

[BITS 32]
PrintCompat:
    ; Disable the interrupts:
    ; Already disabled

    ; 2. Turn off paging
    mov eax, cr0
    and eax, ~PG
    mov cr0, eax

    mov eax, 0
    mov cr3, eax

    ; 3. Use GDT with 16-bit tables (skip this step if one is already available):
    ; Our GDT does have 16 bit code and data segments

    ; 4. Far jump to 16-bit protected mode:
    jmp 0x18:PrintReal

[BITS 64]
LongMode:
    mov rdx, Print

; Rust code is aligned to 16 so we make the end of our file also aligned to 16
ALIGN 16
Rust:
