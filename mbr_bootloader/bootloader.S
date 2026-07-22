BITS 16             ; assemble in 16-bit real mode instruction encoding

SECTOR_1_ADDR equ 0x7C00
PARTITIONS_ADDR equ SECTOR_1_ADDR
MBR_ADDR equ 0x7A00
LBA_ADDR_ADDR equ MBR_ADDR - 12
BUFFER_ADDR equ LBA_ADDR_ADDR - 26

section .stage_0 vstart=0x7C00
stage_0_start:
    cli
    xor ax, ax      ; ax = 0 (cheap way to zero a register, see earlier explanation)
    mov ds, ax      ; ds = 0 — so [addr] means physical address 'addr', not addr+something
    mov es, ax
    mov ss, ax      ; ss = 0 — stack segment, paired with sp below
    mov sp, BUFFER_ADDR

    cld             ; clear direction flag — makes movsb increment si/di (copy forward), not decrement
    ; Copy 256 x u16 from 0x7C00 to 0x7A00
    mov si, 0x7C00
    mov di, 0x7A00
    mov cx, 256
    rep movsw

    ; Jump to start of next stage. We explicitly set CS:IP to 0x0000:stage1_start
    jmp 0x0000:stage_1_start
stage_0_end:

section .stage_1 vstart=(0x7A00 + ($ - $$)) align=1
stage_1_start:
    ; Make sure that the 0x42 extension (which lets us read with LBA addressing) exists
    mov ah, 0x41
    mov bx, 0x55AA
    int 0x13
    jc error_checking_extensions
    cmp bx, 0xAA55
    jne error_extensions_not_present
    test cx, 0x4
    jz error_edd_not_present

    ; Read all MBR partitions to find one with type 0xEE, meaning this disk is GPT
    ; We only support GPT disks
    xor bx, bx
    mov cx, 4
    .check_mbr_entry:
        mov al, [MBR_ADDR + bx + 0x1C2]
        cmp al, 0xEE
        je .is_gpt
        add bx, 16
        loop .check_mbr_entry
    jmp error_not_gpt

    .is_gpt:
    ; Read sector 1
    mov si, msg_reading_gpt_header
    call print

    mov di, LBA_ADDR_ADDR
    mov ax, SECTOR_1_ADDR
    stosw
    xor eax, eax
    stosw
    inc eax
    stosd
    dec eax
    stosd
    call read_sector
    jc error_read_sector_1

    mov bp, [SECTOR_1_ADDR + 0x50]

    ; Read starting LBA of the entries
    ; Reuse the buffer
    mov si, SECTOR_1_ADDR + 0x48
    mov di, LBA_ADDR_ADDR + 0x4
    mov cx, 4
    rep movsw

    .read_entries_sector:
        mov si, msg_reading_gpt_partitions
        call print
        call read_sector
        jc error_read_entries

        ; Use bx to keep track of offset within sector
        xor bx, bx
    .read_entry:
        xor di, di
        mov cx, 8
        mov si, PARTITIONS_ADDR
        add si, bx
    .check_if_zero:
        lodsw
        or di, ax
        loop .check_if_zero
        ; If di is zero, all 16 bytes were zero and this entry is empty
        jz .next

        mov ah, [PARTITIONS_ADDR + bx + 0x30]
        ; Check for bit 2 which means BIOS bootable
        test ah, 0x4
        ; If bit 2 isn't there, don't do anything
        jnz .boot_partition
    .next:
        dec bp
        jz no_bootable_partition
        add bx, 128
        cmp bx, 512
        jl .read_entry
        add dword [LBA_ADDR_ADDR + 0x4], 1
        adc dword [LBA_ADDR_ADDR + 0x8], 0
        jmp .read_entries_sector

    .boot_partition:
        mov si, msg_reading_partition
        call print
        ; Read the first sector of the partition
        mov word [LBA_ADDR_ADDR], 0x7C00
        mov si, PARTITIONS_ADDR + 0x20
        add si, bx
        mov di, LBA_ADDR_ADDR + 0x4
        mov cx, 4
        rep movsw
        call read_sector
        jc error_read_partition

        ; Jump to the partition's BIOS code
        mov si, msg_jumping_to_partition
        call print
        ; Set magic
        ; MAGIC will be passed as an input to nasm
        mov eax, MAGIC
        mov ebx, [LBA_ADDR_ADDR + 0x4]
        mov ecx, [LBA_ADDR_ADDR + 0x8]
        jmp 0x7C00

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

msg_reading_gpt_header db "G_", 0
msg_reading_gpt_partitions db "P_", 0
msg_reading_partition db "A_", 0
msg_jumping_to_partition db "J_", 0x0D, 0x0A, 0

; Error EDD not present
msg_edd         db "DD", 0
; Error extensions not present
msg_ext         db "EE", 0
; Error checking extensions
msg_check       db "CC", 0
; Error reading sector 1
msg_read1       db "11", 0
; Error reading entries
msg_entries     db "NN", 0
; Error reading Partition
msg_part        db "RR", 0
; No bootable partition
msg_bootflag    db "00", 0
; Not a GPT disk
msg_err_not_gpt db "LL", 0

print:
    lodsb
    test al, al
    jz .done
    mov ah, 0x0E
    int 0x10
    jmp print
.done:
    ret

read_sector:
    ; Packet size, must be 16
    mov byte [BUFFER_ADDR], 16
    ; Resrved, must be 0
    mov byte [BUFFER_ADDR + 0x1], 0
    ; Number of sectors to transfer
    mov word [BUFFER_ADDR + 0x2], 1
    ; Destination offset
    ; Destination segment
    ; Starting LBA (low 32 bits)
    ; Starting LBA (high 32 bits)
    mov si, LBA_ADDR_ADDR
    mov di, BUFFER_ADDR + 0x4
    mov cx, 6
    rep movsw
    mov si, BUFFER_ADDR
    mov ah, 0x42
    int 0x13
    ret

stage_1_end:

%if ((stage_0_end - stage_0_start) + (stage_1_end - stage_1_start)) > 420
    %error "Bootloader exceeds 420 bytes"
%endif
