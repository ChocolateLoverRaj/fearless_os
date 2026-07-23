; FIRST_SECTOR_ADDR will be externally supplied
; STAGE_0_SIZE will be externally supplied
; STAGE_2_ADDR will be externally supplied
; STAGE_2_LEN will be externally supplied
ORG FIRST_SECTOR_ADDR + STAGE_0_SIZE
BITS 16


Start:
        mov si, Buffer
        mov ah, 0x42
        int 0x13
        jc ErrorReading

        ; Jump to the next stage
        jmp 0x0:STAGE_2_ADDR

ErrorReading:
        jmp $

ALIGN 2
Buffer:
        ; Buffer len
        db 16
        ; Reserved, must be 0
        db 0
        ; # of blocks to transfer
        db (STAGE_2_LEN + 0x200 - 1) / 0x200
        ; Reserved, must be 0
        db 0
        ; Destination offset
        dw STAGE_2_ADDR
        ; Destination segment
        dw 0
        ; Starting LBA (64 bits)
        dq 1

times 0x1DA - (STAGE_0_SIZE + ($ - $$)) db 0x67

Gdt:
    .Null:
        dq 0x0000000000000000             ; Null Descriptor - should be present.
    .Code:
        dq 0x00209A0000000000             ; 64-bit code descriptor (exec/read).
    .Data:
        dq 0x0000920000000000             ; 64-bit data descriptor (read/write).
    .End:

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

dw 0xAA55
