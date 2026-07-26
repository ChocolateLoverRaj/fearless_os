use std::{
    env::current_dir,
    process::{Command, Stdio},
};

use scripts::{PAGE_TABLES_ADDR, STAGE_2_ADDR};

fn main() {
    let output = Command::new("nasm")
        .current_dir(current_dir().unwrap().parent().unwrap())
        .arg("stage_2.nasm")
        .arg(format!("-DSTAGE_2_ADDR={STAGE_2_ADDR:#X}"))
        .arg(format!("-DPAGE_TABLES_ADDR={PAGE_TABLES_ADDR:#X}"))
        .arg("-f")
        .arg("bin")
        .arg("-o")
        .arg("build/stage_2.bin")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}
