use std::{
    fs,
    io::{Seek, SeekFrom, Write},
};

fn main() {
    let test_bin = fs::read("build/test.bin").unwrap();

    const TOTAL_BYTES: usize = 512 * 68;
    let mut mem_device = std::io::Cursor::new(vec![0u8; TOTAL_BYTES]);
    //
    // Create a protective MBR at LBA0
    let mut mbr = gpt::mbr::ProtectiveMBR::with_lb_size(
        u32::try_from((TOTAL_BYTES / 512) - 1).unwrap_or(0xFF_FF_FF_FF),
    );
    mbr.set_bootcode({
        let mut code = fs::read("build/bootloader.bin").unwrap();
        code.resize(440, Default::default());
        code.try_into().unwrap()
    });
    mbr.overwrite_lba0(&mut mem_device)
        .expect("failed to write MBR");

    let mut gdisk = gpt::GptConfig::default()
        .writable(true)
        .logical_block_size(gpt::disk::LogicalBlockSize::Lb512)
        .create_from_device(mem_device, None)
        .unwrap();

    // At this point, gdisk.primary_header() and gdisk.backup_header() are populated...
    let (partition_lba, length_lba) = gdisk.find_free_sectors()[0];
    let partition_id = gdisk.find_next_partition_id().unwrap();
    gdisk
        .add_partition_at(
            "BOOT",
            partition_id,
            partition_lba,
            length_lba,
            gpt::partition_types::EFI,
            gpt::partition::PartitionAttributes::BOOTABLE.bits(),
        )
        .unwrap();
    // Write the partition table and take ownership of
    // the underlying memory buffer-backed block device
    let mut mem_device = gdisk.write().expect("failed to write partition table");
    mem_device
        .seek(SeekFrom::Start(partition_lba * 512))
        .unwrap();
    mem_device.write_all(&test_bin).unwrap();
    fs::write("build/gpt.img", mem_device.into_inner()).unwrap();
}
