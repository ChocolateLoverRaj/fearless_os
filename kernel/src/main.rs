#![no_std]
#![no_main]

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
mod limine_x86_64;

#[cfg(target_os = "uefi")]
mod uefi;

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "riscv64")]
mod riscv64;
