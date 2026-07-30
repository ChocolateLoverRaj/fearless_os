use std::{
    fs::{self, File},
    process::{Command, Stdio},
};

use elf::{ElfStream, endian::LittleEndian};
use scripts::{FIRST_SECTOR_ADDR, STAGE_2_ADDR};

fn main() {
    let stage_0_size = fs::metadata("../build/stage_0.bin").unwrap().len();
    let stage_2_len = fs::metadata("../build/stage_2.bin").unwrap().len()
        + fs::metadata("../build/rust.bin").unwrap().len();
    let mut elf =
        ElfStream::<LittleEndian, _>::open_stream(File::open("../build/rust").unwrap()).unwrap();
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
    // let rust_start = find_symbol("__rust_start").unwrap();
    // let data_end = find_symbol("__data_end").unwrap();
    let bss_end = find_symbol("__bss_end").unwrap();

    let kib_needed = u16::try_from((bss_end + 0x400 - 1) / 0x400).unwrap();

    let output = Command::new("nasm")
        .arg("../stage_1.nasm")
        .arg(format!("-DFIRST_SECTOR_ADDR={FIRST_SECTOR_ADDR:#X}"))
        .arg(format!("-DSTAGE_0_SIZE={stage_0_size:#X}"))
        .arg(format!("-DSTAGE_2_ADDR={STAGE_2_ADDR:#X}"))
        .arg(format!("-DSTAGE_2_FILE_LEN={stage_2_len:#X}"))
        .arg(format!("-DKIB_NEEDED={kib_needed:#X}"))
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("../build/stage_1.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
