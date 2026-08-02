use std::{
    fmt::UpperHex,
    fs::{self, File},
    process::{Command, Stdio},
};

use elf::{ElfStream, endian::LittleEndian};

use common::{PAGE_TABLE_1G, PAGE_TABLE_256T, PAGE_TABLE_512G, SECTOR_1};

fn main() {
    let util_len = fs::metadata("build/util.bin").unwrap().len();
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
    let util_len_aligned = util_len.next_multiple_of(16);
    let start = find_symbol("__start").unwrap();
    let data_end = find_symbol("__data_end").unwrap();
    let bss_end = find_symbol("__bss_end").unwrap();
    let next_stage_mem_len = util_len_aligned + bss_end - start;
    let next_stage_file_len = util_len_aligned + data_end - start;
    let jmp_addr = find_symbol("_start").unwrap();
    println!("mem len: {next_stage_mem_len:#X}");
    println!("file len: {next_stage_file_len:#X}");
    println!("sector 1 jmp addr: {jmp_addr:#X}");

    let nasm_args: &[(&str, &dyn UpperHex)] = &[
        ("NEXT_STAGE_MEM_LEN", &next_stage_mem_len),
        ("NEXT_STAGE_FILE_LEN", &next_stage_file_len),
        ("NEXT_STAGE_JMP_ADDR", &jmp_addr),
        ("NEXT_STAGE_ADDR", &SECTOR_1),
        ("PAGE_TABLE_256T_ADDR", &PAGE_TABLE_256T),
        ("PAGE_TABLE_512G_ADDR", &PAGE_TABLE_512G),
        ("PAGE_TABLE_1G_ADDR", &PAGE_TABLE_1G),
    ];

    let mut command = Command::new("nasm");
    command.arg("sector_0.nasm");
    for (name, value) in nasm_args {
        command.arg(format!("-D{name}={value:#X}"));
    }
    let output = command
        .arg("-f")
        .arg("bin")
        .arg("-l")
        .arg("build/sector_0.lst")
        .arg("-o")
        .arg("build/sector_0.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
