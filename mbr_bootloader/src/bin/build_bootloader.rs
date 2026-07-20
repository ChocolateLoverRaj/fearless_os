use std::process::{Command, Stdio};

use mbr_bootloader::MAGIC;

fn main() {
    let output = Command::new("nasm")
        .arg("bootloader.S")
        .arg(format!("-DMAGIC={MAGIC}"))
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("build/bootloader.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
