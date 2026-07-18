use std::{
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
};

fn main() {
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open("build/gpt_large.img")
        .unwrap();
    // 4 TiB
    let disk_size = 0x40000000000;
    file.set_len(disk_size).unwrap();
    // Create a protective MBR at LBA0
    let mut mbr = gpt::mbr::ProtectiveMBR::with_lb_size(
        u32::try_from((disk_size / 512) - 1).unwrap_or(u32::MAX),
    );
    mbr.set_bootcode({
        let mut code = fs::read("build/bootloader.bin").unwrap();
        code.resize(440, Default::default());
        code.try_into().unwrap()
    });
    mbr.overwrite_lba0(&mut file).expect("failed to write MBR");

    let mut gdisk = gpt::GptConfig::default()
        .writable(true)
        .logical_block_size(gpt::disk::LogicalBlockSize::Lb512)
        .create_from_device(file, None)
        .unwrap();

    // At this point, gdisk.primary_header() and gdisk.backup_header() are populated...
    let (partition_lba, length_lba) = gdisk.find_free_sectors()[0];
    let partition_id = gdisk.find_next_partition_id().unwrap();
    // In LBAs
    let partition_len = 1;
    let first_lba = partition_lba + (length_lba - partition_len);
    gdisk
        .add_partition_at(
            "BOOT",
            partition_id,
            first_lba,
            partition_len,
            gpt::partition_types::EFI,
            gpt::partition::PartitionAttributes::BOOTABLE.bits(),
        )
        .unwrap();
    // Write the partition table and take ownership of
    // the underlying memory buffer-backed block device
    let mut file = gdisk.write().expect("failed to write partition table");
    file.seek(SeekFrom::Start(first_lba * 512)).unwrap();
    let test_bin = fs::read("build/test.bin").unwrap();
    file.write_all(&test_bin).unwrap();
}
