use std::{
    fmt::Display,
    fs::{self, File},
    path::PathBuf,
};

use common::SECTOR_1;
use elf::{ElfStream, endian::LittleEndian};

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let linker_file = dir.join("linker.ld");
    let linker_file = linker_file.to_str().unwrap();

    // Tell cargo to pass the linker script to the linker..
    println!("cargo:rustc-link-arg=-T{linker_file}");
    // ..and to re-run if it changes.
    println!("cargo:rerun-if-changed={linker_file}");

    let util_path = dir.parent().unwrap().join("build/util.bin");
    let util_len = fs::metadata(&util_path).unwrap().len();
    let load_addr = u64::from(SECTOR_1) + util_len.next_multiple_of(16);
    println!("cargo:rustc-link-arg=--defsym=LOAD_ADDR={load_addr:#X}");
    let util_path = util_path.to_str().unwrap();
    println!("cargo:rerun-if-changed={util_path}");

    let big_stage_path = dir.parent().unwrap().join("build/big_stage");
    let mut elf =
        ElfStream::<LittleEndian, _>::open_stream(File::open(&big_stage_path).unwrap()).unwrap();
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
    let jmp_addr = find_symbol("_start").unwrap();
    println!("mem len: {next_stage_mem_len:#X}");
    println!("file len: {next_stage_file_len:#X}");
    println!("sector 1 jmp addr: {jmp_addr:#X}");

    let env_vars: &[(&str, &dyn Display)] = &[
        ("NEXT_STAGE_MEM_LEN", &next_stage_mem_len),
        ("NEXT_STAGE_FILE_LEN", &next_stage_file_len),
        ("NEXT_STAGE_JMP_ADDR", &jmp_addr),
    ];
    for (key, value) in env_vars {
        println!("cargo:rustc-env={key}={value}");
    }

    let big_stage_path = big_stage_path.to_str().unwrap();
    println!("cargo:rerun-if-changed={big_stage_path}");
}
