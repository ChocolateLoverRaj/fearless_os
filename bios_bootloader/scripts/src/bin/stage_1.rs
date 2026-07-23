use std::{
    fs,
    process::{Command, Stdio},
};

use scripts::{FIRST_SECTOR_ADDR, STAGE_2_ADDR};

fn main() {
    let stage_0_size = fs::metadata("build/stage_0.bin").unwrap().len();
    let stage_2_len = fs::metadata("build/stage_2.bin").unwrap().len()
        + fs::metadata("build/rust.bin").unwrap().len();
    let output = Command::new("nasm")
        .arg("stage_1.nasm")
        .arg(format!("-DFIRST_SECTOR_ADDR={FIRST_SECTOR_ADDR:#X}"))
        .arg(format!("-DSTAGE_0_SIZE={stage_0_size:#X}"))
        .arg(format!("-DSTAGE_2_ADDR={STAGE_2_ADDR:#X}"))
        .arg(format!("-DSTAGE_2_LEN={stage_2_len:#X}"))
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("build/stage_1.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
