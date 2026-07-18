BITS 16
ORG 0x7C00

start:
    mov al, 0x10
    mov dx, 0xf4
    out dx, al
    ; if isa-debug-exit isn't present/doesn't work, fall through to a safe halt
    cli
    hlt
    jmp $

times 510 - ($ - $$) db 0
dw 0xAA55
