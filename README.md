## Folder Structure
### Bootloaders
`bios_bootloader` is a custom Legacy BIOS bootloader specifically for booting Fearless OS through a custom handoff.

`kernel` is where the actual kernel goes. It includes entry points from bootloaders.

`android_deploy` is a development tool, see its README.

## Supported Devices
Note that similar devices may also work, but I'm listing the ones I tested / target. Feel free to contribute to this list and OS.

### x86_64
- QEMU `pc`
- QEMU `q35`
- Lenovo Z560 Laptop
- Robo360 Chromebook

### aarch64
- QEMU `virt`
- QEMU `raspi3b`
- QEMU `raspi4b`
- Raspberry Pi 3B

### arm
- QEMU `virt`
- QEMU `raspi0`
- QEMU `raspi1ap`
- QEMU `raspi2b`

### riscv64
- QEMU `virt`

## Features
### Legacy BIOS
- Hello World
- Shutdown with ACPI

### UEFI
- Hello World

### Raspberry Pi
- Timer interrupts

## Latest development
Handling ACPI events, particularly power button press.

## The name
- Fearless inspired by the song [Fearless on NCS](https://ncs.io/fearless2)
- One of the main goals is to be able to run untrusted programs without worrying that they will spy on you or mess up your system or files
