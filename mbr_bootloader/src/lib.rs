use std::{
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
    os::unix::fs::FileExt,
};

pub const MAGIC: u32 = 0xA786B9FC;
pub const DEBUG_EXIT_VALUE: u8 = 0x10;

/// Partition index starts at 1 and last valid one is 128
pub fn build_test_image(name: &str, disk_size: u64, gpt_partition_index: u32) {
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(format!("build/{name}.img"))
        .unwrap();
    file.set_len(disk_size).unwrap();
    file.write_all_at(&fs::read("build/bootloader.bin").unwrap(), 0)
        .unwrap();
    gptman::GPT::write_protective_mbr_into(&mut file, 512).unwrap();
    let mut gpt = gptman::GPT::new_from(
        &mut file,
        512,
        [
            0x3A, 0xF1, 0x07, 0xC4, 0x9D, 0x22, 0x6B, 0xE8, 0x14, 0x75, 0xA0, 0x5F, 0xD3, 0x8C,
            0x19, 0xB6,
        ],
    )
    .unwrap();
    gpt.align = 1;
    let (free_start, free_len) = gpt.find_free_sectors()[0];
    let partition_len = 1;
    let first_lba = free_start + (free_len - partition_len);
    gpt[gpt_partition_index] = gptman::GPTPartitionEntry {
        partition_type_guid: [
            0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11, 0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E,
            0xC9, 0x3B,
        ],
        unique_partition_guid: [
            0x91, 0x6D, 0x4E, 0x2B, 0xA7, 0x53, 0xC1, 0x4F, 0x8E, 0xD2, 0x6A, 0x39, 0xF0, 0xB5,
            0x7C, 0x14,
        ],
        starting_lba: first_lba,
        ending_lba: first_lba + partition_len - 1,
        attribute_bits: 0x4,
        partition_name: "BOOT".into(),
    };
    gpt.write_into(&mut file).unwrap();
    file.seek(SeekFrom::Start(first_lba * 512)).unwrap();
    let test_bin = fs::read("build/test.bin").unwrap();
    file.write_all(&test_bin).unwrap();
}
