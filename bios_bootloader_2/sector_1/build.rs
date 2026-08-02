use std::{fs, path::PathBuf};

use common::SECTOR_1;

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let linker_file = dir.join("linker.ld");
    let linker_file = linker_file.to_str().unwrap();

    // Tell cargo to pass the linker script to the linker..
    println!("cargo:rustc-link-arg=-T{linker_file}");
    // ..and to re-run if it changes.
    println!("cargo:rerun-if-changed={linker_file}");

    let util_path = dir.parent().unwrap().join("build/util.bin");
    let util_len = fs::metadata(&util_path).unwrap().len();
    let load_addr = u64::from(SECTOR_1) + util_len.next_multiple_of(16);
    println!("cargo:rustc-link-arg=--defsym=LOAD_ADDR={load_addr:#X}");
    let util_path = util_path.to_str().unwrap();
    println!("cargo:rerun-if-changed={util_path}");
}
