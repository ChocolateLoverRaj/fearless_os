ORG 0x7C00
BITS 16

; KIB_NEEDED will be externally supplied
; FIRST_SECTOR_ADDR will be externally supplied
; STACK_TOP_ADDR will be externally supplied

Start:
        cli
        jmp 0x0000:AfterReloadCs

AfterReloadCs:
        xor ax, ax
        mov ss, ax
        mov sp, STACK_TOP_ADDR
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
        cld

        ; Make sure that the 0x42 extension exists
        mov ah, 0x41
        mov bx, 0x55AA
        int 0x13
        jc ErrorCheckingExtensions
        cmp bx, 0xAA55
        jne ErrorExtensionsNotPresent
        test cx, 0x4
        jz ErrorEddNotPresent

        ; Check if there is enough low memory
        int 0x12
        jc ErrorGettingMemory
        cmp ax, KIB_NEEDED
        jl ErrorNotEnoughMem

        ; Copy self
        mov si, 0x7C00
        mov di, FIRST_SECTOR_ADDR
        mov cx, 256
        rep movsw

        ; Jump to self down
        jmp FIRST_SECTOR_ADDR + (End - Start)

ErrorCheckingExtensions:
        jmp $

ErrorExtensionsNotPresent:
        jmp $

ErrorEddNotPresent:
        jmp $

ErrorGettingMemory:
        jmp $

ErrorNotEnoughMem:
        jmp $

End:
