BITS 16
; STAGE_0_SIZE will be passed from the code that calls `nasm`
ORG 0x7A00 + STAGE_0_SIZE

start:
    ; Set stack pointer right below where we are copied
    mov sp, 0x7A00

    ; Make sure that the 0x42 extension (which lets us read with LBA addressing) exists
    mov ah, 0x41
    mov bx, 0x55AA
    int 0x13
    jc error_checking_extensions
    cmp bx, 0xAA55
    jne error_extensions_not_present
    and cx, 0x4
    jz error_edd_not_present

    ; Read sector 1
    mov si, msg_reading_gpt_header
    call print
    SECTOR_1_ADDR equ 0x7C00
    ; Packet size, must be 16
    mov byte [buffer], 16
    ; Resrved, must be 0
    mov byte [buffer + 1], 0
    ; Number of sectors to transfer
    mov word [buffer + 2], 1
    ; Destination offset
    mov word [buffer + 4], SECTOR_1_ADDR
    ; Destination segment
    mov word [buffer + 6], 0
    ; Starting LBA (low 32 bits)
    mov dword [buffer + 8], 1
    ; Starting LBA (high 32 bits)
    mov dword [buffer + 12], 0
    mov si, buffer
    mov ah, 0x42
    int 0x13
    jc error_read_sector_1

    ; Read starting LBA of the entries
    ; Reuse the buffer
    mov si, msg_reading_gpt_partitions
    call print
    PARTITIONS_ADDR equ SECTOR_1_ADDR
    mov eax, [SECTOR_1_ADDR + 0x48]
    mov [buffer + 8], eax
    mov eax, [SECTOR_1_ADDR + 0x4C]
    mov [buffer + 12], eax
    mov si, buffer
    mov ah, 0x42
    int 0x13
    jc error_read_entries

    ; Read the first partition entry
    mov si, msg_reading_partition
    call print
    mov ah, [PARTITIONS_ADDR + 0x30]
    ; Check for bit 2 which means BIOS bootable
    and ah, 0x4
    ; If bit 2 isn't there, don't do anything
    jz not_bios_bootable

    ; Read the first sector of the partition
    ; Reuse the buffer
    mov eax, [PARTITIONS_ADDR + 0x20]
    mov [buffer + 8], eax
    mov eax, [PARTITIONS_ADDR + 0x24]
    mov [buffer + 12], eax
    mov si, buffer
    mov ah, 0x42
    int 0x13
    jc error_read_partition

    ; Jump to the partition's BIOS code
    mov si, msg_jumping_to_partition
    call print
    jmp 0x7C00

    ; Read sector size
    ; mov word [buffer], 26
    ; mov si, buffer
    ; mov ah, 0x48
    ; int 0x13
    ; jc error
    ; Store bytes per sector in ax
    ; mov ax, [buffer + 24]

error_checking_extensions:
    mov si, msg_check
    call print
    jmp $

error_extensions_not_present:
    mov si, msg_ext
    call print
    jmp $

error_edd_not_present:
    mov si, msg_edd
    call print
    jmp $

error_read_sector_1:
    mov si, msg_read1
    call print
    jmp $

error_read_entries:
    mov si, msg_entries
    call print
    jmp $

error_read_partition:
    mov si, msg_part
    call print
    jmp $

not_bios_bootable:
    mov si, msg_bootflag
    call print
    jmp $

buffer:
    times 26 db 0

msg_reading_gpt_header db "G", 0x0D, 0x0A, 0
msg_reading_gpt_partitions db "P", 0x0D, 0x0A, 0
msg_reading_partition db "A", 0x0D, 0x0A, 0
msg_jumping_to_partition db "J", 0x0D, 0x0A, 0

msg_edd         db "ED", 0x0D, 0x0A, 0
msg_ext         db "EE", 0x0D, 0x0A, 0
msg_check       db "EC", 0x0D, 0x0A, 0
msg_read1       db "EG", 0x0D, 0x0A, 0
msg_entries     db "EP", 0x0D, 0x0A, 0
msg_part        db "EA", 0x0D, 0x0A, 0
msg_bootflag    db "EN", 0x0D, 0x0A, 0

print:
    lodsb
    test al, al
    jz .done
    mov ah, 0x0E
    int 0x10
    jmp print
.done:
    ret
