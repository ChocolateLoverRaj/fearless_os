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

#[cfg(target_arch = "arm")]
mod arm;

mod bcm2835_aux;
mod bcm2835_aux_uart;
mod bcm2836_armctrl_ic;
mod bcm2836_l1_intc;
