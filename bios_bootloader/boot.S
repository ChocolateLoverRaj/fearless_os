ORG 0x7C00
BITS 16

Start:
        cli
        ; Some BIOS' may load us at 0x0000:0x7C00 while other may load us at 0x07C0:0x0000.
        ; Do a far jump to fix this issue, and reload CS to 0x0000.
        jmp 0x0000:AfterReloadCs

AfterReloadCs:
        xor ax, ax
        mov ss, ax
        mov sp, Start
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
        ; clear the direction flag
        cld

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
        int 0x12
        jc ErrorGettingMemory
        ; ax = number of KiB of memory available, starting from address 0
        ; We'll need 12 KiB for 3 page tables
        cmp ax, 12
        jl NotEnoughMem

        ; Copy the pointer to the GDT and the GDT
        mov si, GdtPointer
        mov di, 0xFD6
        mov cx, 17
        repe movsw

        ; Load the GDT
        lgdt [0xFD6]

        ; Create the (empty) IDT
        xor ax, ax
        mov cx, 4
        rep stosw

        ; Load the IDT
        lidt [0xFF8]

        ; Create page tables, identity-mapping the bottom 1 GiB
        PRESENT equ 1 << 0
        WRITABLE equ 1 << 1
        USER_ACCESSIBLE equ 1 << 2
        WRITE_THROUGH equ 1 << 3
        NO_CACHE equ 1 << 4
        PAGE_SIZE equ 1 << 7

        ; Create the top level page table
        ; Skip page at 0x0 because Rust doesn't allow null pointer operations
        ; Create the first entry
        ; Point to 8 KiB address
        mov eax, PRESENT | WRITABLE | ((0x2000 >> 12) << 12)
        stosd
        ; Zero the rest
        xor ax, ax
        mov cx, 0x7FE
        rep stosw

        ; Create the next level page table
        mov eax, PRESENT | WRITABLE | ((0x3000 >> 12) << 12)
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
        mov eax, 0x1000
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

GdtPointer:
    ; Size of GDT - 1
    dw 0x17
    ; Address of GDT
    dq 0xFE0

Gdt:
    .Null:
        dq 0x0000000000000000             ; Null Descriptor - should be present.
    .Code:
        dq 0x00209A0000000000             ; 64-bit code descriptor (exec/read).
    .Data:
        dq 0x0000920000000000             ; 64-bit data descriptor (read/write).

[BITS 64]
LongMode:
    jmp $
