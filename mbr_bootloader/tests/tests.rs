use std::fs::metadata;
use std::process::{Command, Stdio};

#[test]
fn fits() {
    assert!(metadata("build/bootloader.bin").unwrap().len() <= 440);
}

#[test]
fn boots_gpt_2() {
    let output = Command::new("timeout")
        .arg("5")
        .arg("qemu-system-x86_64")
        .arg("--machine")
        .arg("pc,accel=kvm:whpx:hvf:tcg")
        .arg("--no-reboot")
        .arg("--nographic")
        .arg("-drive")
        .arg("file=build/gpt.img,format=raw,if=ide,snapshot=on")
        .arg("-device")
        .arg("isa-debug-exit,iobase=0xf4,iosize=0x04")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 33);
}
