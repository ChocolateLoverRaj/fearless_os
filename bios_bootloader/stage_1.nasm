; FIRST_SECTOR_ADDR will be externally supplied
; STAGE_0_SIZE will be externally supplied
; STAGE_2_ADDR will be externally supplied
; STAGE_2_LEN will be externally supplied
ORG FIRST_SECTOR_ADDR + STAGE_0_SIZE
BITS 16

Start:
        ; ecx = sectors left to read
        mov ecx, (STAGE_2_LEN + 0x200 - 1) / 0x200
    .Loop:
        test ecx, ecx
        jz .Done
        ; Sectors to read
        mov eax, ecx
        movzx esi, word [Buffer.DestOffset]
        shr esi, 9
        mov ebx, 128
        sub ebx, esi
        ; now ebx = max sectors until we hit 64 KiB boundary
        cmp eax, ebx
        jbe .WithinBoundary
        mov eax, ebx
    .WithinBoundary:
        ; At this point eax = sectors to read (max 128)
        cmp eax, 127
        jbe .SmallEnough
        mov eax, 127
    .SmallEnough:
        ; At this point, eax = sectors to read (max 127)
        mov [Buffer.TransferCount], al
        mov si, msg
        call Print
        mov si, Buffer
        pushad
        mov ah, 0x42
        int 0x13
        jc ErrorReading
        popad

        ; Update sectors left to read
        sub ecx, eax

        ; Advance the starting LBA
        ; We are assuming that we will not be reading past 4 GiB so just 32 bits is ok
        add [Buffer.StartingLba], eax
        ; Advance the dest offset
        shl ax, 9
        add [Buffer.DestOffset], ax
        ; Advance the dest segment if needed
        jnc .AfterAdvanceSegment
        add word [Buffer.DestSegment], 0x1000
    .AfterAdvanceSegment:
        jmp .Loop

    .Done:
        mov si, msg_done
        call Print
        ; Jump to the next stage
        jmp 0x0:STAGE_2_ADDR

ErrorReading:
        jmp $

ALIGN 2
Buffer:
    .Len
        ; Buffer len
        db 16
    .Reserved
        ; Reserved, must be 0
        db 0
    .TransferCount
        ; # of blocks to transfer
        db (STAGE_2_LEN + 0x200 - 1) / 0x200
    .Reserved2
        ; Reserved, must be 0
        db 0
    .DestOffset
        ; Destination offset
        dw STAGE_2_ADDR
    .DestSegment
        ; Destination segment
        dw 0
    .StartingLba
        ; Starting LBA (64 bits)
        dq 1

msg db "Reading", 0x0D, 0x0A, 0
msg_done db "Jumping to stage 2", 0x0D, 0x0A, 0

Print:
    pushad
    .Loop:
        lodsb
        test al, al
        jz .Done

        mov ah, 0x0E
        mov bh, 0
        int 0x10

        jmp .Loop

    .Done:
        popad
        ret
