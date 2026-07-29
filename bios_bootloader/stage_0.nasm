ORG 0x7C00
BITS 16

; KIB_NEEDED will be externally supplied
; FIRST_SECTOR_ADDR will be externally supplied
; STACK_TOP_ADDR will be externally supplied

Start:
        ; Workaround for some BIOSes that require this stub
        jmp SkipBpb
        nop
; Some BIOSes will do a funny and decide to overwrite bytes of code in
; the section where a FAT BPB would be, potentially overwriting
; bootsector code.
; Avoid that by filling the BPB area with dummy values.
; Some of the values have to be set to certain values in order
; to boot on even quirkier machines.
; Source: https://github.com/freebsd/freebsd-src/blob/82a21151cf1d7a3e9e95b9edbbf74ac10f386d6a/stand/i386/boot2/boot1.S
Bpb:
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
SkipBpb:
    cli
    cld
    jmp 0x0:AfterReloadCs

AfterReloadCs:
        xor ax, ax
        mov ss, ax
        mov sp, STACK_TOP_ADDR
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
        sti

        mov si, msg
        call Print

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
        mov si, Start
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

msg db "Hello from Stage 0", 0x0D, 0x0A, 0

Print:
    .Loop:
        lodsb
        test al, al
        jz .Done

        mov ah, 0x0E
        mov bh, 0
        int 0x10

        jmp .Loop

    .Done:
        ret

End:
