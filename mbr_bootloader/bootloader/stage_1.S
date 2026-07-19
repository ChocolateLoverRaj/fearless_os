BITS 16
; STAGE_0_SIZE will be passed from the code that calls `nasm`
ORG 0x7A00 + STAGE_0_SIZE

MBR_ADDR equ 0x7A00

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

    ; Read MBR partition 1 -> partition type
    mov al, [MBR_ADDR + 0x1C2]
    cmp al, 0xEE
    jne error_not_gpt

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
    mov si, SECTOR_1_ADDR + 0x48
    mov di, buffer + 0x8
    mov cx, 4
    rep movsw

    .read_entries_sector:
        mov si, msg_reading_gpt_partitions
        call print
        PARTITIONS_ADDR equ SECTOR_1_ADDR
        mov si, buffer
        mov ah, 0x42
        int 0x13
        jc error_read_entries

        ; Use ebx to keep track of offset within sector
        xor ebx, ebx
        xor edi, edi
    .read_entry:
        mov cx, 4
        mov esi, PARTITIONS_ADDR
        add esi, ebx
    .check_if_zero:
        lodsd
        or edi, eax
        loop .check_if_zero
        ; If edi is zero, all 16 bytes were zero and this entry is empty
        jz .next

        mov ah, [PARTITIONS_ADDR + ebx + 0x30]
        ; Check for bit 2 which means BIOS bootable
        test ah, 0x4
        ; If bit 2 isn't there, don't do anything
        jnz .boot_partition
    .next:
        add ebx, 128
        cmp ebx, 512
        jl .read_entry
        ; Check if this is the last one
        mov si, buffer + 0x8
        mov di, SECTOR_1_ADDR + 0x30
        mov cx, 4
        repe cmpsw
        je no_bootable_partition
        add dword [buffer + 0x8], 1
        adc dword [buffer + 0xC], 0
        jmp .read_entries_sector

    .boot_partition:
        mov si, msg_reading_partition
        call print
        ; Read the first sector of the partition
        ; Reuse the buffer
        mov esi, PARTITIONS_ADDR + 0x20
        add esi, ebx
        mov di, buffer + 0x8
        mov cx, 4
        rep movsw
        mov si, buffer
        mov ah, 0x42
        int 0x13
        jc error_read_partition

    ; Jump to the partition's BIOS code
    mov si, msg_jumping_to_partition
    call print
    ; Set magic
    ; MAGIC will be passed as an input to nasm
    mov eax, MAGIC
    mov ebx, [buffer + 0x8]
    mov ecx, [buffer + 0xC]
    jmp 0x7C00

    ; Read sector size
    ; mov word [buffer], 26
    ; mov si, buffer
    ; mov ah, 0x48
    ; int 0x13
    ; jc error
    ; Store bytes per sector in ax
    ; mov ax, [buffer + 24]

error_not_gpt:
    mov si, msg_err_not_gpt
    jmp error

error_checking_extensions:
    mov si, msg_check
    jmp error

error_extensions_not_present:
    mov si, msg_ext
    jmp error

error_edd_not_present:
    mov si, msg_edd
    jmp error

error_read_sector_1:
    mov si, msg_read1
    jmp error

error_read_entries:
    mov si, msg_entries
    jmp error

error_read_partition:
    mov si, msg_part
    jmp error

no_bootable_partition:
    mov si, msg_bootflag
; must set si to error message before this
error:
    call print
    jmp $

buffer:
    times 26 db 0

msg_reading_gpt_header db "G", 0x0D, 0x0A, 0
msg_reading_gpt_partitions db "P", 0x0D, 0x0A, 0
msg_reading_partition db "A", 0x0D, 0x0A, 0
msg_jumping_to_partition db "J", 0x0D, 0x0A, 0

; Error EDD not present
msg_edd         db "D", 0
; Error extensions not present
msg_ext         db "E", 0
; Error checking extensions
msg_check       db "C", 0
; Error reading sector 1
msg_read1       db "1", 0
; Error reading entries
msg_entries     db "N", 0
; Error reading Partition
msg_part        db "R", 0
; No bootable partition
msg_bootflag    db "0", 0
; Not a GPT disk
msg_err_not_gpt db "L", 0

print:
    lodsb
    test al, al
    jz .done
    mov ah, 0x0E
    int 0x10
    jmp print
.done:
    ret
