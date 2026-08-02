# Memory Layout
0-0x5000 - BIOS memory, must be preserved
0x5000-0x6000 - Top level page table (points to 256 TiB)
0x6000-0x7000 - Page table (points to 512 GiB)
0x7000-0x7C00 - Stack
0x7C00 - Partition Sector 0 loaded by BIOS
0x8000-0x9000 - Page table (points to 1 GiB)
0x9000 - 32-bit Rust code

# File Layout
Protective MBR
GPT

Partition:
Sector 0 (512 B)
Sector 1 (starts at sector 1, up to 604 KiB)

GPT backup
