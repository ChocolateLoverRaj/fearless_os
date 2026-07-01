#![cfg_attr(target_arch = "arm", feature(stdarch_arm_neon_intrinsics))]
mod entry;
mod interrupts;
mod logger;
mod panic_handler;

use core::ptr::{NonNull, addr_of};

use aarch64_cpu::{
    asm::wfi,
    registers::{
        CurrentEL, DAIF, ELR_EL2, HCR_EL2, MPIDR_EL1, Readable, SP_EL1, SPSR_EL2, Writeable,
    },
};
use log::info;

use self::logger::uart;
use super::bcm2836_armctrl_ic::Bcm2836ArmCtrlIc;

unsafe extern "C" {
    static __kernel_start: u8;
    static __stack_top: u8;
}

pub unsafe extern "C" fn kernel_main(fdt_addr: usize) -> ! {
    unsafe { logger::init() };

    let el = CurrentEL
        .read_as_enum::<CurrentEL::EL::Value>(CurrentEL::EL)
        .unwrap();
    let kernel_addr = addr_of!(__kernel_start).addr();
    let mpidr = MPIDR_EL1.get();
    info!(
        "Hello from the kernel! Loaded at {kernel_addr:#X}, FDT Addr: {fdt_addr:#X}, EL: {el:?}, mpidr: {mpidr:#X}"
    );

    // The Linux boot protocol says that the kernel is started in either EL2 or EL1
    // Since we're not doing hypervisor stuff, drop down to EL1 if we're in EL2
    if el == CurrentEL::EL::Value::EL2 {
        // Set EL1 execution state to AArch64.
        HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);

        // Configure that eret will go into EL1 with interrupts disabled
        SPSR_EL2.write(
            SPSR_EL2::D::Masked
                + SPSR_EL2::A::Masked
                + SPSR_EL2::I::Masked
                + SPSR_EL2::F::Masked
                + SPSR_EL2::M::EL1h,
        );

        // Configure to jump to this fn on eret
        ELR_EL2.set(kernel_main_el1 as *const () as u64);

        // Configure the stack pointer on eret
        let stack_top = addr_of!(__stack_top).addr();
        SP_EL1.set(stack_top as u64);

        info!("Dropping to EL1");
        unsafe {
            core::arch::asm!("eret", in("x0") fdt_addr, options(nomem, nostack, noreturn));
        }
    } else {
        kernel_main_el1(fdt_addr);
    }

    // loop {
    //     let byte = uart.read_sync();
    //     uart.write_sync_no_flush(byte.to_ascii_uppercase());
    // }
}

extern "C" fn kernel_main_el1(fdt_addr: usize) -> ! {
    let el = CurrentEL
        .read_as_enum::<CurrentEL::EL::Value>(CurrentEL::EL)
        .unwrap();
    assert_eq!(el, CurrentEL::EL::Value::EL1);
    let daif = DAIF.get();
    info!("Hello from EL1. DAIF: {daif:#X}");

    unsafe { interrupts::init() };

    let rx_enabled = uart().try_lock().unwrap().rx_interrupt_enabled();
    info!("RX int enabled: {rx_enabled}");
    let tx_enabled = uart().try_lock().unwrap().tx_interrupt_enabled();
    info!("TX int enabled: {tx_enabled}");

    uart().try_lock().unwrap().set_rx_interrupt_enabled(true);
    uart().try_lock().unwrap().set_tx_interrupt_enabled(false);

    let rx_enabled = uart().try_lock().unwrap().rx_interrupt_enabled();
    info!("RX int enabled: {rx_enabled}");
    let tx_enabled = uart().try_lock().unwrap().tx_interrupt_enabled();
    info!("TX int enabled: {tx_enabled}");

    let mut ic = unsafe { Bcm2836ArmCtrlIc::new(NonNull::new(0x3F00_B200 as *mut _).unwrap()) };
    let aux_int_en = ic.get_interrupt_enabled(29);
    info!("AUX int enabled: {aux_int_en}");
    ic.set_interrupt_enabled(29, true);
    let aux_int_en = ic.get_interrupt_enabled(29);
    info!("AUX int enabled: {aux_int_en}");

    let aux_int_pending = ic.is_pending(29);
    info!("AUX int pending: {aux_int_pending}");
    let rx_int_pending = uart().try_lock().unwrap().rx_interrupt_pending();
    info!("RX int pending: {rx_int_pending}");
    let tx_int_pending = uart().try_lock().unwrap().tx_interrupt_pending();
    info!("TX int pending: {tx_int_pending}");

    // Enable interrupts
    DAIF.write(DAIF::D::CLEAR + DAIF::A::CLEAR + DAIF::I::CLEAR + DAIF::F::CLEAR);
    loop {
        wfi();
    }
}
