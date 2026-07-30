use std::process::{Command, Stdio};

fn main() {
    const MAGIC: u32 = 0xA786B9FC;
    let output = Command::new("nasm")
        .arg("../../mbr_bootloader/bootloader.nasm")
        .arg(format!("-DMAGIC={MAGIC}"))
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("../build/mbr_bootloader.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
