use std::{
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
};

use mbrman::{BOOT_ACTIVE, CHS, MBR};

fn main() {
    // Checks
    let first_sector_len = fs::metadata("../build/stage_0.bin").unwrap().len()
        + fs::metadata("../build/stage_1.bin").unwrap().len();
    assert!(first_sector_len < 440);

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("../build/disk.img")
        .unwrap();
    // Make it 1 MiB, minimum required by qemu q35 to boot
    file.set_len(0x100000).unwrap();
    // let mut mbr = MBR::new_from(&mut file, 512, [0x1, 0x2, 0x3, 0x4]).unwrap();
    let stage_0 = fs::read("../build/stage_0.bin").unwrap();
    file.write_all(&stage_0).unwrap();
    let stage_1 = fs::read("../build/stage_1.bin").unwrap();
    file.write_all(&stage_1).unwrap();
    // let boot_code = [stage_0, stage_1].concat();
    // mbr.header.bootstrap_code[..boot_code.len()].copy_from_slice(&boot_code);
    // mbr.header.boot_signature = [0x55, 0xAA];
    // mbr[1] = mbrman::MBRPartitionEntry {
    //     boot: BOOT_ACTIVE,
    //     first_chs: CHS::empty(),
    //     sys: 0xDA,
    //     last_chs: CHS::empty(),
    //     starting_lba: 1,
    //     sectors: 2047,
    // };
    // mbr.write_into(&mut file).unwrap();
    // file.seek(SeekFrom::Start(0x200)).unwrap();
    file.seek(SeekFrom::Start(510)).unwrap();
    file.write_all(&[0x55, 0xAA]).unwrap();
    let stage_2 = fs::read("../build/stage_2.bin").unwrap();
    file.write_all(&stage_2).unwrap();
    let stage_rust = fs::read("../build/rust.bin").unwrap();
    file.write_all(&stage_rust).unwrap();
}
