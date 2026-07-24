use std::{fs, path::PathBuf};

use scripts::STAGE_2_ADDR;

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let linker_file = dir.join("linker.ld");
    let linker_file = linker_file.to_str().unwrap();

    // Tell cargo to pass the linker script to the linker..
    println!("cargo:rustc-link-arg=-T{linker_file}");
    // ..and to re-run if it changes.
    println!("cargo:rerun-if-changed={linker_file}");

    let stage_2_path = dir.parent().unwrap().join("build/stage_2.bin");
    let stage_2_len = fs::metadata(&stage_2_path).unwrap().len();
    let load_addr = u64::from(STAGE_2_ADDR) + stage_2_len;
    println!("cargo:rustc-link-arg=--defsym=LOAD_ADDR={load_addr:#X}");
    println!("cargo:warning={load_addr:#X}");
    let stage_2_path = stage_2_path.to_str().unwrap();
    println!("cargo:rereun-if-changed={stage_2_path}");
}
