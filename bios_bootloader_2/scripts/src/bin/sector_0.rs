use std::{
    fs::File,
    process::{Command, Stdio},
};

use elf::{ElfStream, endian::LittleEndian};

fn main() {
    let mut elf =
        ElfStream::<LittleEndian, _>::open_stream(File::open("build/sector_1").unwrap()).unwrap();
    let (symbol_table, string_table) = elf.symbol_table().unwrap().unwrap();

    let find_symbol = |symbol: &str| {
        symbol_table.iter().find_map(|s| {
            if let Ok(str) = string_table.get(s.st_name as usize)
                && str == symbol
            {
                Some(s.st_value)
            } else {
                None
            }
        })
    };
    let start = find_symbol("__start").unwrap();
    let data_end = find_symbol("__data_end").unwrap();
    let bss_end = find_symbol("__bss_end").unwrap();
    let next_stage_mem_len = bss_end - start;
    let next_stage_file_len = data_end - start;
    let start_addr = find_symbol("_start").unwrap();

    let output = Command::new("nasm")
        .arg("sector_0.nasm")
        .arg(format!("-DNEXT_STAGE_MEM_LEN={next_stage_mem_len:#X}"))
        .arg(format!("-DNEXT_STAGE_FILE_LEN={next_stage_file_len:#X}"))
        .arg(format!("-DNEXT_STAGE_JMP_ADDR={start_addr:#X}"))
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("build/sector_0.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
