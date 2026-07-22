use std::{
    fs,
    process::{Command, Stdio},
};

fn main() {
    let next_stage_size = fs::metadata("build/stage_2.bin").unwrap().len()
        + fs::metadata("build/rust.bin").unwrap().len();
    let output = Command::new("nasm")
        .arg("stage_0.nasm")
        .arg(format!("-DFILE_LEN={next_stage_size:#X}"))
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("build/stage_0.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
