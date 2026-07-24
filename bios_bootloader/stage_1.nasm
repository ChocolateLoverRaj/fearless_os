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
