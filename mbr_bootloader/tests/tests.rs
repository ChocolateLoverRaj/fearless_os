use std::fs::metadata;
use std::process::{Command, Stdio};

use mbr_bootloader::MAGIC;

#[test]
fn fits() {
    assert!(metadata("build/bootloader.bin").unwrap().len() <= 440);
}

fn test_img(img: &str, partition_lba: u64) {
    let output = Command::new("timeout")
        .arg("5")
        .arg("qemu-system-x86_64")
        .arg("--machine")
        .arg("pc,accel=kvm:whpx:hvf:tcg")
        .arg("--no-reboot")
        .arg("--nographic")
        .arg("-drive")
        .arg(format!("file=build/{img},format=raw,if=ide,snapshot=on"))
        .arg("-device")
        .arg("isa-debug-exit,iobase=0xf4,iosize=0x04")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    let expected_eax = MAGIC;
    let expected_ebx = partition_lba as u32;
    let expected_exc = (partition_lba >> 32) as u32;
    let expected_str = format!(
        "{:08X}{:08X}{:08X}",
        expected_eax.to_be(),
        expected_ebx.to_be(),
        expected_exc.to_be()
    );
    assert_eq!(output.status.code().unwrap(), 33);
    let test_output = str::from_utf8(&output.stdout)
        .unwrap()
        .lines()
        .last()
        .unwrap()
        .split_at(expected_str.len())
        .0;
    assert_eq!(test_output, expected_str);
}

#[test]
fn boots_gpt_small() {
    test_img("gpt.img", 0x22);
}

#[test]
fn boots_gpt_large() {
    test_img("gpt_large.img", 0x1FFFFFFDE);
}
