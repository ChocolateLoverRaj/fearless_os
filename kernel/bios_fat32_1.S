BITS 16
ORG 0x7C5A

start:
    xor ax, ax      ; ax = 0 (cheap way to zero a register, see earlier explanation)
    mov ds, ax      ; ds = 0 — so [addr] means physical address 'addr', not addr+something
    mov es, ax      ; es = 0 — needed since rep movsb uses es:di for the destination
    mov ss, ax      ; ss = 0 — stack segment, paired with sp below
    ; Set stack pointer right below where we are copied
    mov sp, 0x7C00

    ; ebx = hidden sectors
    mov ebx, [0x7C1C]

    ; eax = reserved sectors
    movzx eax, word [0x7C0E]
    ; ebx = hidden sectors + reserved sectors
    add ebx, eax

    ; add (num_fats * sectors_per_fat32)
    ; eax = number of FATs
    movzx eax, byte [0x7C10]
    ; ecx = sectors per FAT
    mov ecx, [0x7C24]
    ; eax = number of FATs * sectors per FAT
    mul ecx
    ; ebx = sector on disk of first cluster
    add ebx, eax

    ; ecx = root dir first cluster
    mov ecx, [0x7C2C]
    ; ecx = root dir first cluster - 2 (actual first cluster, since 0 and 1 are reserved)
    sub ecx, 2

    ; eax = sectors_per_cluster
    movzx eax, byte [0x7C0D]
    ; eax = sector from start of sectors of root dir
    mul ecx
    ; eax = actual first sector of root dir
    add eax, ebx

    ; Read the root dir
    DIR_SECTOR_ADDR equ 0x7A00
    ; Packet size, must be 16
    mov byte [buffer], 16
    ; Resrved, must be 0
    mov byte [buffer + 1], 0
    ; Number of sectors to transfer
    mov word [buffer + 2], 1
    ; Destination offset
    mov word [buffer + 4], DIR_SECTOR_ADDR
    ; Destination segment
    mov word [buffer + 6], 0
    ; Starting LBA (low 32 bits)
    mov dword [buffer + 8], 1
    ; Starting LBA (high 32 bits)
    mov dword [buffer + 12], 0
    mov si, buffer
    mov ah, 0x42
    int 0x13
    jc error_read_sector
    jmp $

    mov dword [entry_ptr], DIR_SECTOR_ADDR

check_entry:
    mov si, [entry_ptr]
    ; Load attributes
    mov bl, [si + 11]
    ; Make sure it's not a directory or a volume ID
    and bl, 0x18
    jnz next_entry
    ; Check if the name is "BOOT.BIN"
    mov di, target_name
    mov cx, 11
    repe cmpsb
    jne next_entry

    ; Found it — extract starting cluster
    mov si, [entry_ptr]
    mov ax, [si + 20]           ; high word
    mov dx, [si + 26]            ; low word
    shl eax, 16
    mov ax, dx
    mov eax, 0x67
    jmp $


next_entry:
    add dword [entry_ptr], 32
    jmp check_entry
    ; TODO: read next sector
    ; TODO: read next cluster

entry_ptr:
    dd 0x7A00

buffer:
    times 26 db 0

target_name: db "BOOT    BIN"

error_read_sector:
    jmp $
