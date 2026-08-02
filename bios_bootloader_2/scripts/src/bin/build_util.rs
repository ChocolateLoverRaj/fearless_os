use std::process::{Command, Stdio};

use common::SECTOR_1;

fn main() {
    let output = Command::new("nasm")
        .arg("util.nasm")
        .arg(format!("-DSELF_ADDR={SECTOR_1:#X}"))
        .arg("-l")
        .arg("build/util.lst")
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("build/util.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
