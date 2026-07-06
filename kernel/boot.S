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

    cld
    mov si, message

.print:
    lodsb
    test al, al
    jz .done

    mov ah, 0x0E
    mov bh, 0          ; page number
    mov bl, 0x07       ; text attribute (ignored in text modes)
    int 0x10

    jmp .print

.done:
    cli
.hang:
    hlt
    jmp .hang

message:
    db "Hello, World!", 0x0D, 0x0A, 0

times 510 - ($ - $$) db 0
dw 0xAA55
