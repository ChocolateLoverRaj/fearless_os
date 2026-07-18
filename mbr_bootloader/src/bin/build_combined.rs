use std::fs;

fn main() {
    fs::write(
        "build/bootloader.bin",
        ["build/stage_0.bin", "build/stage_1.bin"]
            .map(|file| fs::read(file).unwrap())
            .concat(),
    )
    .unwrap();
}
