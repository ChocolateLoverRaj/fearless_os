; ------------------------------------------------------------
; Minimal bootable MBR (512 bytes)
; Prints "Hello World" using BIOS teletype
; Includes a fake partition table entry
; ------------------------------------------------------------

BITS 16
ORG 0x7C00

start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti

    mov si, msg

.print:
    lodsb
    test al, al
    jz hang
    mov ah, 0x0E
    int 0x10
    jmp .print

hang:
    cli
.hang:
    hlt
    jmp .hang


; ------------------------------------------------------------
; Data
; ------------------------------------------------------------
msg db "Hello World!", 0x0D, 0x0A, 0


; ------------------------------------------------------------
; Pad boot code to 446 bytes (MBR boot region)
; ------------------------------------------------------------
times 446 - ($ - $$) db 0


; ------------------------------------------------------------
; Fake partition table (1 entry, 16 bytes)
; ------------------------------------------------------------
db 0x80                ; bootable flag
db 0x01, 0x01, 0x00    ; CHS start (fake)
db 0x0C                ; partition type (FAT32 LBA)
db 0xFF, 0xFF, 0xFF    ; CHS end (fake)
dd 1                   ; LBA start
dd 2                   ; number of sectors


; second/third/fourth entries (empty)
times 16 * 3 db 0


; ------------------------------------------------------------
; Boot signature
; ------------------------------------------------------------
dw 0xAA55
