BITS 16             ; assemble in 16-bit real mode instruction encoding

SECTOR_1_ADDR equ 0x7C00
PARTITIONS_ADDR equ SECTOR_1_ADDR
MBR_ADDR equ 0x7A00
LBA_ADDR_ADDR equ MBR_ADDR - 12
BUFFER_ADDR equ LBA_ADDR_ADDR - 26

section .stage_0 vstart=0x7C00
stage_0_start:
        ; Workaround for some BIOSes that require this stub
        jmp skip_bpb
        nop

        ; Some BIOSes will do a funny and decide to overwrite bytes of code in
        ; the section where a FAT BPB would be, potentially overwriting
        ; bootsector code.
        ; Avoid that by filling the BPB area with dummy values.
        ; Some of the values have to be set to certain values in order
        ; to boot on even quirkier machines.
        ; Source: https://github.com/freebsd/freebsd-src/blob/82a21151cf1d7a3e9e95b9edbbf74ac10f386d6a/stand/i386/boot2/boot1.S
bpb:
        times 3-($-$$) db 0
    .bpb_oem_id:            db "LIMINE  "
    .bpb_sector_size:       dw 512
    .bpb_sects_per_cluster: db 0
    .bpb_reserved_sects:    dw 0
    .bpb_fat_count:         db 0
    .bpb_root_dir_entries:  dw 0
    .bpb_sector_count:      dw 0
    .bpb_media_type:        db 0
    .bpb_sects_per_fat:     dw 0
    .bpb_sects_per_track:   dw 18
    .bpb_heads_count:       dw 2
    .bpb_hidden_sects:      dd 0
    .bpb_sector_count_big:  dd 0
    .bpb_drive_num:         db 0
    .bpb_reserved:          db 0
    .bpb_signature:         db 0
    .bpb_volume_id:         dd 0
    .bpb_volume_label:      db "LIMINE     "
    .bpb_filesystem_type:   times 8 db 0
skip_bpb:
    cli
    cld
    jmp 0x0:after_reload_cs

after_reload_cs:
    xor si, si
    mov ss, si
    mov sp, BUFFER_ADDR
    mov ds, si
    mov es, si
    mov fs, si
    mov gs, si

    ; Set error char to '0'
    mov dh, 0x30

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
    ; Print 'H'
    mov al, 0x48
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
        ; Print 'R'
        mov al, 0x50
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
        ; Print 'R'
        mov al, 0x52
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
        ; Print 'J'
        mov al, 0x4A
        call print
        ; Print new line
        mov al, 0x0D
        call print
        mov al, 0x0A
        call print
        ; Set magic
        ; MAGIC will be passed as an input to nasm
        mov eax, MAGIC
        mov ebx, [LBA_ADDR_ADDR + 0x4]
        mov ecx, [LBA_ADDR_ADDR + 0x8]
        jmp 0x7C00

error_not_gpt:
    inc dh
error_checking_extensions:
    inc dh
error_extensions_not_present:
    inc dh
error_edd_not_present:
    inc dh
error_read_sector_1:
    inc dh
error_read_entries:
    inc dh
error_read_partition:
    inc dh
error_bootflag:
    inc dh
no_bootable_partition:
    inc dh
error:
    mov ah, 0x0E
    mov al, dh
    int 0x10
    int 0x10
    jmp $

print:
    mov ah, 0x0E
    int 0x10
    int 0x10
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
