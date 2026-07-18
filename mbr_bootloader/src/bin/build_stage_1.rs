use std::{
    fs,
    process::{Command, Stdio},
};

use mbr_bootloader::MAGIC;

fn main() {
    let stage_0_size = fs::metadata("build/stage_0.bin").unwrap().len();
    let output = Command::new("nasm")
        .arg("bootloader/stage_1.S")
        .arg(format!("-DSTAGE_0_SIZE={stage_0_size:#X}"))
        .arg(format!("-DMAGIC={MAGIC}"))
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("build/stage_1.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
