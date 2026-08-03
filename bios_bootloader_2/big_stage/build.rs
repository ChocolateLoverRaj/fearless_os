use std::path::PathBuf;

use common::BIG_STAGE_ADDR;

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let linker_file = dir.join("linker.ld");
    let linker_file = linker_file.to_str().unwrap();

    // Tell cargo to pass the linker script to the linker..
    println!("cargo:rustc-link-arg=-T{linker_file}");
    // ..and to re-run if it changes.
    println!("cargo:rerun-if-changed={linker_file}");

    println!("cargo:rustc-link-arg=--defsym=LOAD_ADDR={BIG_STAGE_ADDR:#X}");
}
