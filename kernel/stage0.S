BITS 16             ; assemble in 16-bit real mode instruction encoding
ORG 0x7C00          ; BIOS loads and runs us at physical address 0x7C00

start:
    xor ax, ax      ; ax = 0 (cheap way to zero a register, see earlier explanation)
    mov ds, ax      ; ds = 0 — so [addr] means physical address 'addr', not addr+something
    mov es, ax      ; es = 0 — needed since rep movsb uses es:di for the destination
    mov ss, ax      ; ss = 0 — stack segment, paired with sp below

    cld             ; clear direction flag — makes movsb increment si/di (copy forward), not decrement
    mov si, 0x7C00  ; source index = 0x7C00, i.e. our own current location in memory
    mov di, 0x7A00  ; destination index = 0x7A00, i.e. 512 bytes below where we are now
    mov cx, 512     ; counter = 512 — we're copying our whole sector, byte by byte
    rep movsb       ; repeat: copy [ds:si] -> [es:di], si++, di++, cx--, until cx == 0

    ; Jump to start of next stage. We explicitly set CS:IP to 0x0000:stage1_start
    jmp 0x0000:(stage1_start - 512)

stage1_start:
    ; Will be copied here from separate file
