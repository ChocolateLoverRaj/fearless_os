use std::{
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
};

use gptman::GPT;
use mbrman::{BOOT_ACTIVE, CHS, MBR};

fn main() {
    let sector_1_len = fs::metadata("build/sector_1.bin").unwrap().len();
    let partition_sectors_count = 1 + (sector_1_len + 512 - 1) / 512;
    let disk_sectors_count = (1 + 1 + 32 + partition_sectors_count + 32 + 1).max(0x800);
    let disk_len = disk_sectors_count * 512;
    let mut disk = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("build/disk.img")
        .unwrap();
    disk.set_len(disk_len).unwrap();

    // Create GPT tables
    let mut gpt = GPT::new_from(
        &mut disk,
        512,
        [
            0x02, 0xb0, 0x15, 0xdb, 0x78, 0x30, 0xae, 0x19, 0x46, 0x8d, 0x48, 0x92, 0x50, 0xa9,
            0x0d, 0xf2,
        ],
    )
    .unwrap();
    gpt.align = 1;
    let starting_lba = gpt.find_first_place(partition_sectors_count).unwrap();
    gpt[1] = gptman::GPTPartitionEntry {
        partition_type_guid: [
            0xb3, 0x0a, 0xb1, 0x01, 0xb2, 0xdb, 0xff, 0x9a, 0xa4, 0xd5, 0x1d, 0x46, 0x83, 0x69,
            0x3c, 0x43,
        ],
        unique_partition_guid: [
            0x0e, 0x9f, 0x09, 0xfe, 0xbd, 0xdd, 0x55, 0x6d, 0x3a, 0xd0, 0xcf, 0x8b, 0xce, 0x97,
            0x1f, 0x99,
        ],
        starting_lba,
        ending_lba: starting_lba + partition_sectors_count - 1,
        attribute_bits: 0x04,
        partition_name: "Fearless OS".into(),
    };
    gpt.write_into(&mut disk).unwrap();

    // Create a protective MBR
    let mut mbr = MBR::new_from(&mut disk, 512, 0x5b9cc9ca_u32.to_le_bytes()).unwrap();
    let boot_code = fs::read("build/disk_sector_0.bin").unwrap();
    mbr.header.bootstrap_code[..boot_code.len()].copy_from_slice(&boot_code);
    mbr.header.boot_signature = [0x55, 0xAA];
    mbr[1] = mbrman::MBRPartitionEntry {
        boot: BOOT_ACTIVE,
        first_chs: CHS::new(0, 0, 2),
        sys: 0xEE,
        last_chs: CHS::new(0xFFFF, 0xF, 0xF),
        starting_lba: 1,
        sectors: (disk_sectors_count - 1).try_into().unwrap_or(u32::MAX),
    };
    mbr.write_into(&mut disk).unwrap();

    // Create the partition
    disk.seek(SeekFrom::Start(starting_lba * 512)).unwrap();
    let partition_sector_0 = fs::read("build/sector_0.bin").unwrap();
    disk.write_all(&partition_sector_0).unwrap();
    disk.seek(SeekFrom::Start((starting_lba + 1) * 512))
        .unwrap();
    let util = fs::read("build/util.bin").unwrap();
    disk.write_all(&util).unwrap();
    let padding = 16 - (util.len() % 16);
    disk.seek_relative(padding.try_into().unwrap()).unwrap();
    let partition_sector_1 = fs::read("build/sector_1.bin").unwrap();
    disk.write_all(&partition_sector_1).unwrap();
}
