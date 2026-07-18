use std::process::{Command, Stdio};

fn main() {
    let output = Command::new("nasm")
        .arg("bootloader/stage_0.S")
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("build/stage_0.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
