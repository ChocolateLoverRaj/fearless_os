#![no_std]
#![no_main]

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
mod limine_x86_64;

#[cfg(target_os = "uefi")]
mod uefi;
