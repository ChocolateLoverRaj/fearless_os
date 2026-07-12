ORG 0x7C00
BITS 16

ALIGN 4
IDT:
    .Length       dw 0
    .Base         dd 0

Start:
    ; Some BIOS' may load us at 0x0000:0x7C00 while other may load us at 0x07C0:0x0000.
    ; Do a far jump to fix this issue, and reload CS to 0x0000.
    jmp 0x0000:.Next

.Next:
    ; ax = 0
    xor ax, ax
    ; ss = 0
    mov ss, ax
    ; Set up stack so that it starts below where we are loadedj.
    mov sp, Start
    ; make ds, es, fs, gs = 0
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    ; clear the direction flag
    cld

    ; Get memory
    xor ebx, ebx
.GetMemoryMap:
    mov di, .MemoryMapBuffer
    mov edx, 0x534D4150
    mov eax, 0xE820
    mov ecx, 24
    int 0x15
    jc .ErrorCheckingMemory
    cmp eax, 0x0534D4150
    jc .ErrorCheckingMemory

    mov si, GotEntry
    call Print

    ; Check if it's free memory
    mov eax, [.MemoryMapBuffer + 0x10]
    cmp eax, 0x1
    jne .MaybeLoop

    ; Check if it's >4 GiB
    mov eax, [.MemoryMapBuffer + 0x14]
    test eax, eax
    ; If it is, we don't care if it's the largest, 4 GiB is enough
    jz .CompareLenLower32
    ; Make it so we don't continue looping
    xor ebx, ebx
    jmp .UpdateLargest

.CompareLenLower32:
    ; Check if it's more than the existing
    mov eax, [.MemoryMapBuffer + 0x8]
    cmp eax, [.LargestFreeMemLen]
    jb .MaybeLoop

.UpdateLargest:
    ; Set source ptr
    mov si, .MemoryMapBuffer
    ; Set dest ptr
    mov di, .LargestFreeMemStart
    ; Do the thing 4 times
    mov cx, 4
    ; Copy a u32 from src to dest and advance both src and dest ptrs, <cx> times
    rep movsd

.MaybeLoop:
    ; If ebx is 0, stop searching
    test ebx, ebx
    jnz .GetMemoryMap

.NoMoreMemory:
    ; Check whether we support Long Mode or not.
    ; Check whether CPUID is supported or not.
    ; It's supported, bit 0x200000 can be changed
    ; This original one will be preserved
    pushfd
    ; This one is just to pop immediately
    pushfd
    ; Toggle the bit
    xor dword [esp], 0x200000
    popfd
    ; Read it back and see if it's still there
    pushfd
    pop eax
    ; See which bits changed
    xor eax, [esp]
    ; Restore original eflags
    popfd
    ; If the bit changed, that means CPUID is supported
    test eax, 0x200000
    jz .NoCpuId

    ; CpuId is supported.
    ; Use this CPUID to check the highest CPUID function implemented
    mov eax, 0x80000000
    cpuid
    ; We need the function 0x80000001, so the highest must be at least this
    cmp eax, 0x80000001
    jb .NoExtendedFunction
    ; Get feature flags
    mov eax, 0x80000001
    cpuid
    ; Bit 29 is long mode
    test edx, 1 << 29
    jz .NoLongMode

    ; Long mode is supported.
    mov     ax, 0x2403              ; Query A20 gate support
    int     0x15
    jc      .a20_ns                  ; INT 0x15 is not supported
    test    ah, ah
    jnz     .a20_ns                  ; INT 0x15 is not supported

    mov     ax, 0x2402              ; Get A20 gate status
    int     0x15
    jc      .a20_failed              ; Couldn't get status
    test    ah, ah
    jnz     .a20_failed              ; Couldn't get status
    test    al, al
    jnz     .a20_activated           ; AL = 1, A20 gate is already activated

    mov     ax, 0x2401              ; Activate A20 gate
    int     0x15
    jc      .a20_failed              ; Couldn't activate the gate
    test    ah, ah
    jnz     .a20_failed              ; Couldn't activate the gate

.a20_activated:                  ; Go on
    jmp $

.a20_ns:
    jmp $

.a20_failed:
    jmp $

.ErrorCheckingMemory:
    jmp $

.NoCpuId:
    jmp $

.NoExtendedFunction:
    jmp $

.NoLongMode:
    jmp $

ALIGN 8
.MemoryMapBuffer:
    times 24 db 0

.LargestFreeMemStart:
    times 8 db 0
.LargestFreeMemLen:
    times 8 db 0

Print:
    pushad
.PrintLoop:
    lodsb                             ; Load the value at [@es:@si] in @al.
    test al, al                       ; If AL is the terminator character, stop printing.
    je .PrintDone
    mov ah, 0x0E
    int 0x10
    jmp .PrintLoop                    ; Loop till the null character not found.

.PrintDone:
    popad                             ; Pop all general purpose registers to save them.
    ret

GotEntry db "Got Memory Map Entry", 0x0A, 0x0D, 0

gdt_start:
    ; Null descriptor — mandatory, entry 0
    dq 0x0000000000000000
gdt_code:
    ; 64-bit code segment
    ; base=0, limit=0 (ignored in long mode)
    ; access byte: present=1, DPL=0, type=1(code/data), exec=1, rw=1 -> 10011010b = 0x9A
    ; flags: L=1, D=0, G=0 -> 0010b, limit high nibble = 0
    dw 0x0000          ; limit 0:15
    dw 0x0000          ; base 0:15
    db 0x00            ; base 16:23
    db 10011010b        ; access byte (0x9A)
    db 00100000b         ; flags (L=1) + limit 16:19 (0)
    db 0x00               ; base 24:31
gdt_data:
    ; 64-bit data segment
    ; access byte: present=1, DPL=0, type=1, exec=0, rw=1 -> 10010010b = 0x92
    ; flags: not executable, so L bit doesn't apply the same way; conventionally set flags=0
    dw 0x0000
    dw 0x0000
    db 0x00
    db 10010010b         ; access byte (0x92)
    db 00000000b           ; flags — L bit is only meaningful for code segments
    db 0x00
gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1    ; GDT size (limit), must be size-1
    dd gdt_start                  ; GDT base address (32-bit; use dq if loading in true 64-bit context)

CODE_SEG equ gdt_code - gdt_start
DATA_SEG equ gdt_data - gdt_start

times 510 - ($ - $$) db 0
dw 0xAA55
