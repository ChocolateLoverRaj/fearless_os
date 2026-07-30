use std::process::{Command, Stdio};

use scripts::{FIRST_SECTOR_ADDR, STACK_TOP_ADDR};

fn main() {
    let output = Command::new("nasm")
        .arg("../stage_0.nasm")
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
