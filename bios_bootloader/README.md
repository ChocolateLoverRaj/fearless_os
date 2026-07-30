# Memory Layout
0-0x5000 - BIOS memory, must be preserved
0x5000-0x6000 - Stack
0x6000-0x9000 - Page Tables
0x9000-0x9200 - Stage 0, Stage 1 (becomes free after jump to stage 2)
0x9200 - Stage 2, Stage Rust

Then Rust stage decides what to do with the memory

# Disk Layout
Sector 0 - Stage 0, Stage 1
Sector 1.. - Stage 2, Stage Rust
.. - Stage 3

# Possible Improvements
- Format disk with GPT and protective MBR to mark the place where our code will go as unknown type, so that external tools will know something's here.
