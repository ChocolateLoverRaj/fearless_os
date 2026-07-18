use std::process::{Command, Stdio};

use mbr_bootloader::DEBUG_EXIT_VALUE;

fn main() {
    let output = Command::new("nasm")
        .arg("test.S")
        .arg(format!("-DDEBUG_EXIT_VALUE={DEBUG_EXIT_VALUE:#X}"))
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("build/test.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
