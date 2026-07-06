BITS 16
ORG 0x7C00 ; We expect to be loaded at the BIOS standard spot

start_0:
    jmp 0x7C5A
