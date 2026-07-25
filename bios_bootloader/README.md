# Memory Layout
0-0x5000 - BIOS memory, must be preserved
0x5000-0x6000 - Stack
0x6000-0x9000 - Page Tables
0x9000-0x9200 - Stage 0, Stage 1
0x9200 and beyond - Stage 2, Stage Rust
