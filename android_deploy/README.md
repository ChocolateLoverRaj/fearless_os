Many android devices can behave as USB Mass Storage devices, which we can use to develop bootable disks without having to physically unplug them to plug them into another computer. Based off of [isodrive](https://github.com/nitanmarcel/isodrive).

## Requirements
- An Android device that:
  - Supports CONFIGFS to act as a USB device (legacy methods could also work but are currently not supported by this tool)
  - Is rooted or has access to an ADB rooted shell (which can be enabled in LineageOS without the need to "root" your phone). ADB method is currently not supported but it should be really easy to support.
  - You can wirelessly `ssh` or `adb` into.

## Tested Android devices
- Google Pixel 2 XL running LineageOS, rooted with Magisk

## Tested target devices
- Lenovo Z560 Laptop
- Robo360 Chromebook running MrChromebox UEFI full ROM firmware

## Device Setup
### Termux Way
- Root your device.
- Install Termux
- Turn on a SSH server in Termux
- Set up sudo in Termux
- Add your SSH key into your phone's Termux SSH server
- Make sure `rsync` is installed on the server

### LineageOS way
- Install LineageOS.
- Go into developer settings and enable wireless debugging and root in adb
- Connect to your phone with adb and make sure you can run things as root

## Host setup
- Make sure `rsync` is installed

## How to use it
You just need a file which is the raw disk file, such as `build/disk.img`.
```
cargo run -- build/disk.img
```
Make an issue for configuring SSH info and using ADB method.

## How it works
- Development machine runs a simple Rust program
- It copies your file into tmpfs on your phone
- It sends a bash script to run bash commands on your phone
- The bash scripts configures CONFIGFS to serve your raw disk
