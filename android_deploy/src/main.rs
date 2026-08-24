use std::{
    io::Write,
    process::{Command, Stdio},
};

use clap::Parser;

#[derive(Debug, Parser)]
struct Cli {
    file: String,
}

fn main() {
    let cli = Cli::parse();
    let ssh_port = 8022;
    let ssh_addr = "192.168.1.117";
    let ssh_user = "u0_a208";

    // Copy the file to the android device
    println!("Copying file with rsync");
    if !Command::new("rsync")
        .arg("-e")
        .arg("ssh -p 8022")
        .arg("--rsync-path=su -c $(which rsync)")
        .arg(cli.file)
        .arg(format!("{ssh_user}@{ssh_addr}:/tmp/android_deploy"))
        .spawn()
        .unwrap()
        .wait()
        .unwrap()
        .success()
    {
        panic!("Failed to copy file to android device");
    }

    // Run the following on the target
    println!("Configuring CONFIGFS");
    let commands = include_str!("commands.sh");
    let mut child = Command::new("ssh")
        .arg(format!("{ssh_user}@{ssh_addr}"))
        .arg("-p")
        .arg(ssh_port.to_string())
        .arg("su -")
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(include_str!("commands.sh").as_bytes())
        .unwrap();
    if !child.wait().unwrap().success() {
        panic!("Failed to run commands on android device");
    }
}
