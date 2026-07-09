BITS 16
ORG 0x7C5A

DIR_SECTOR_ADDR equ 0x7C00 - 512
FAT_SECTOR_ADDR equ DIR_SECTOR_ADDR - 512
FAT_LBA_ADDR equ FAT_SECTOR_ADDR - 4
DIR_CURSOR_ADDR equ FAT_LBA_ADDR - 4
BUFFER_ADDR equ DIR_CURSOR_ADDR - 26
ENTRY_PTR_ADDR equ BUFFER_ADDR - 4
SECTOR_POS_WITHIN_CLUSTER equ ENTRY_PTR_ADDR - 1
DL_BUFFER_ADDR equ SECTOR_POS_WITHIN_CLUSTER - 1
SECTORS_PER_CLUSTER_ADDR equ DL_BUFFER_ADDR - 1
STACK_TOP equ SECTORS_PER_CLUSTER_ADDR - 1; Must be aligned to 2

start:
    xor ax, ax      ; ax = 0 (cheap way to zero a register, see earlier explanation)
    mov ds, ax      ; ds = 0 — so [addr] means physical address 'addr', not addr+something
    mov es, ax      ; es = 0 — needed since rep movsb uses es:di for the destination
    mov ss, ax      ; ss = 0 — stack segment, paired with sp below
    mov sp, STACK_TOP

    ; save dl because mul modifies it
    mov [DL_BUFFER_ADDR], dl

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
    mov [SECTORS_PER_CLUSTER_ADDR], al
    ; eax = sector from start of sectors of root dir
    mul ecx
    ; eax = actual first sector of root dir
    add eax, ebx

    mov dword [FAT_LBA_ADDR], 0
    mov dword [SECTOR_POS_WITHIN_CLUSTER], 0

read_dir_sector:
    mov [DIR_CURSOR_ADDR], eax
    ; Read the root dir
    ; Packet size, must be 16
    mov byte [BUFFER_ADDR], 16
    ; Resrved, must be 0
    mov byte [BUFFER_ADDR + 1], 0
    ; Number of sectors to transfer
    mov word [BUFFER_ADDR + 2], 1
    ; Destination offset
    mov word [BUFFER_ADDR + 4], DIR_SECTOR_ADDR
    ; Destination segment
    mov word [BUFFER_ADDR + 6], 0
    ; Starting LBA (low 32 bits)
    mov dword [BUFFER_ADDR + 8], eax
    ; Starting LBA (high 32 bits)
    mov dword [BUFFER_ADDR + 12], 0
    mov si, BUFFER_ADDR
    mov ah, 0x42
    mov dl, [DL_BUFFER_ADDR]
    int 0x13
    jc error_read_sector

    mov dword [ENTRY_PTR_ADDR], DIR_SECTOR_ADDR

check_entry:
    mov si, [ENTRY_PTR_ADDR]
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
    mov si, [ENTRY_PTR_ADDR]
    mov ax, [si + 20]           ; high word
    mov dx, [si + 26]           ; low word
    shl eax, 16
    mov ax, dx
    mov eax, 0x67
    jmp $

next_entry:
    add dword [ENTRY_PTR_ADDR], 32
    cmp dword [ENTRY_PTR_ADDR], DIR_SECTOR_ADDR + 512
    jne check_entry
    ; read next sector
    mov al, [SECTOR_POS_WITHIN_CLUSTER]
    inc al
    cmp al, [SECTORS_PER_CLUSTER_ADDR]
    je read_next_cluster
    mov [SECTOR_POS_WITHIN_CLUSTER], al

    mov eax, [DIR_CURSOR_ADDR]
    inc eax
    jmp read_dir_sector

read_next_cluster:
    ; TODO
    jmp $

target_name: db "BOOT    BIN"

error_read_sector:
    jmp $
