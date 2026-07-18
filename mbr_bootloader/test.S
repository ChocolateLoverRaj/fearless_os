BITS 16
ORG 0x7C00

start:
    ; Print eax, ebx, ecx
    mov sp, 0x7C00
    push ecx
    push ebx
    push eax
    mov si, sp
    mov cx, 12

    .loop:
        lodsb
        ; save al
        mov bl, al
        ; print higher 4 bits
        shr al, 4
        call print_hex
        ; print lower 4 bits
        mov al, bl
        and al, 0xF
        call print_hex
        loop .loop

    mov si, msg
    call print

    ; From nasm args
    mov al, DEBUG_EXIT_VALUE
    mov dx, 0xf4
    out dx, al
    ; if isa-debug-exit isn't present/doesn't work, fall through to a safe halt

hlt_loop:
    hlt
    jmp hlt_loop

; Input: lower 4 bits of al, upper 4 bits must be 0
; Like {:X}
print_hex:
        cmp al, 10
        jge .letter
    .decimal:
        ; ASCII "0" is 0x30
        add al, 0x30
        jmp .print
    .letter:
        ; ASCII "A" is0x41
        add al, (0x41 - 10)
    .print:
        mov ah, 0x0E
        int 0x10
        ret

print:
    lodsb
    test al, al
    jz .done
    mov ah, 0x0E
    int 0x10
    jmp print
.done:
    ret

msg db "__", 0x0D, 0x0A, 0

times 510 - ($ - $$) db 0
dw 0xAA55
