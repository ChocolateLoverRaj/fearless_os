ORG 0x7C00
BITS 16

SELF_MOVE_ADDR equ 0x200
FILE_LOAD_ADDR equ 0x400
KB_NEEDED equ (FILE_LOAD_ADDR + FILE_LEN + 0x400 - 1) / 0x400
; FILE_LEN will be externally supplied

Start:
        cli
        jmp 0x0000:AfterReloadCs

AfterReloadCs:
        xor ax, ax
        mov ss, ax
        mov sp, SELF_MOVE_ADDR
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
        cmp ax, KB_NEEDED
        jl ErrorNotEnoughMem

        ; Copy self down
        mov si, 0x7C00
        mov di, SELF_MOVE_ADDR
        mov cx, 256
        rep movsw

        ; Jump to self down
        jmp SELF_MOVE_ADDR + (End - Start)

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
