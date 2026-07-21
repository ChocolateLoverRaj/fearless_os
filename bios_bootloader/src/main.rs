use std::{
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
};

fn main() {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("build/disk.img")
        .unwrap();
    // Make it 1 MiB, minimum required by qemu q35 to boot
    file.set_len(0x100000).unwrap();
    let bootloader = fs::read("build/boot.bin").unwrap();
    file.write_all(&bootloader).unwrap();
    file.seek(SeekFrom::Start(510)).unwrap();
    // Mark it as bootable by BIOS
    file.write_all(&[0x55, 0xAA]).unwrap();
}
