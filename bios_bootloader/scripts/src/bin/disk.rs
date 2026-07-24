use std::{
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
};

fn main() {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("../build/disk.img")
        .unwrap();
    // Make it 1 MiB, minimum required by qemu q35 to boot
    file.set_len(0x100000).unwrap();
    let stage_0 = fs::read("../build/stage_0.bin").unwrap();
    file.write_all(&stage_0).unwrap();
    let stage_1 = fs::read("../build/stage_1.bin").unwrap();
    file.write_all(&stage_1).unwrap();
    file.seek(SeekFrom::Start(510)).unwrap();
    file.write_all(&[0x55, 0xAA]).unwrap();
    let stage_2 = fs::read("../build/stage_2.bin").unwrap();
    file.write_all(&stage_2).unwrap();
    let stage_rust = fs::read("../build/rust.bin").unwrap();
    file.write_all(&stage_rust).unwrap();
}
