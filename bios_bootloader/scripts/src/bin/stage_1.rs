use std::{
    fs,
    process::{Command, Stdio},
};

fn main() {
    let stage_0_size = fs::metadata("build/stage_0.bin").unwrap().len();
    let next_stage_size = fs::metadata("build/stage_2.bin").unwrap().len()
        + fs::metadata("build/rust.bin").unwrap().len();
    let output = Command::new("nasm")
        .arg("stage_1.nasm")
        .arg(format!("-DSTAGE_0_SIZE={stage_0_size:#X}"))
        .arg(format!("-DFILE_LEN={next_stage_size:#X}"))
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("build/stage_1.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
