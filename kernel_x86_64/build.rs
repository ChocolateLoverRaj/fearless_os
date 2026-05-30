use std::path::PathBuf;

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let linker_file = PathBuf::from(dir).join("linker.ld");
    let linker_file = linker_file.to_str().unwrap();

    // Tell cargo to pass the linker script to the linker..
    println!("cargo:rustc-link-arg=-T{linker_file}");
    // ..and to re-run if it changes.
    println!("cargo:rerun-if-changed={linker_file}");
}
