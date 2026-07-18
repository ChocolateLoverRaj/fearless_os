use std::process::{Command, Stdio};

fn main() {
    let output = Command::new("nasm")
        .arg("test.S")
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("build/test.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
