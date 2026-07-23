# Memory Layout
0-0x1000 - Stack
0x1000-0x4000 - Page Tables
0x4000-0x4200 - Stage 0, Stage 1, GDT, IDT, Boot Signature
0x4200 and beyond - Stage 2, Stage Rust
