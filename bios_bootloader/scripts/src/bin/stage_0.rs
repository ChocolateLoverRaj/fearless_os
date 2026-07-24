use std::{
    fs,
    process::{Command, Stdio},
};

use scripts::{FIRST_SECTOR_ADDR, STACK_TOP_ADDR, STAGE_2_ADDR};

fn main() {
    let next_stage_size = fs::metadata("../build/stage_2.bin").unwrap().len()
        + fs::metadata("../build/rust.bin").unwrap().len();
    let kib_needed = (STAGE_2_ADDR + u16::try_from(next_stage_size).unwrap()).div_ceil(0x400);
    let output = Command::new("nasm")
        .arg("../stage_0.nasm")
        .arg(format!("-DKIB_NEEDED={kib_needed:#X}"))
        .arg(format!("-DFIRST_SECTOR_ADDR={FIRST_SECTOR_ADDR:#X}"))
        .arg(format!("-DSTACK_TOP_ADDR={STACK_TOP_ADDR:#X}"))
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("../build/stage_0.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
