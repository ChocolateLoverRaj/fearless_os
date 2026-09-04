use core::str::FromStr;

use acpi::{
    aml::{
        self, AmlError,
        namespace::AmlName,
        object::{Object, WrappedObject},
    },
    registers::Pm1EventFlags,
};
use alloc::vec;
use spin::Once;
use x86_64::{
    instructions::tables::load_tss,
    registers::segmentation::{CS, SS, Segment},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        idt::{InterruptDescriptorTable, InterruptStackFrame},
        tss::TaskStateSegment,
    },
};

use crate::{
    acpi_events::{self, ACPI_GLOBALS, platform},
    apic::end_of_interrupt,
    logger,
};

pub struct Gdt {
    gdt: GlobalDescriptorTable<5>,
    kernel_code_selector: SegmentSelector,
    kernel_data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

static IDT: Once<InterruptDescriptorTable> = Once::new();
static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<Gdt> = Once::new();

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    log::debug!("Breakpoint! Stack frame: {stack_frame:#?}");
    logger().flush();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    log::info!("Timer interrupt!");
    logger().flush();
    unsafe { end_of_interrupt() };
}

extern "x86-interrupt" fn sci_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let events = acpi_events::pending_events();
    acpi_events::clear_events(events);
    log::info!("SCI interrupt! {events:?}.");
    if events.contains(Pm1EventFlags::POWER_BUTTON) {
        let platform = platform();
        let interpreter = &ACPI_GLOBALS.get().unwrap().aml_interpreter;
        let s5 = interpreter
            .evaluate(AmlName::from_str(r#"\_S5_"#).unwrap(), vec![])
            .unwrap();
        let Object::Package(package) = &*s5 else {
            panic!()
        };
        let Object::Integer(slp_type_a) = &*package[0] else {
            panic!()
        };
        let Object::Integer(slp_type_b) = &*package[1] else {
            panic!()
        };
        log::info!("S5: slp_type_a={slp_type_a} slp_type_b={slp_type_b}.");
        match interpreter.evaluate(
            AmlName::from_str(r#"\_PTS"#).unwrap(),
            vec![WrappedObject::new(Object::Integer(5))],
        ) {
            Ok(_) | Err(AmlError::ObjectDoesNotExist(_)) => Ok(()),
            Err(e) => Err(e),
        }
        .unwrap();
        log::info!("Called prepare to sleep.");
        logger().flush();
        platform
            .registers
            .pm1_control_registers
            .set_sleep_typ((*slp_type_a).try_into().unwrap());
        platform
            .registers
            .pm1_control_registers
            .set_bit(acpi::registers::Pm1ControlBit::SleepEnable, true);
        log::info!("Did shutdown. You shouldn't see this");
        logger().flush();
    }
    unsafe { end_of_interrupt() };
}

extern "x86-interrupt" fn ehci_interrupt_handler(_stack_frame: InterruptStackFrame) {
    log::info!("eHCI interrupt!");
    logger().flush();
    unsafe { end_of_interrupt() };
}

pub fn init() {
    let tss = TSS.call_once(TaskStateSegment::new);
    let gdt = GDT.call_once(|| {
        let mut gdt = GlobalDescriptorTable::<5>::empty();
        let kernel_code_selector = gdt.append(Descriptor::kernel_code_segment());
        let kernel_data_selector = gdt.append(Descriptor::kernel_data_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(tss));

        Gdt {
            gdt,
            kernel_code_selector,
            kernel_data_selector,
            tss_selector,
        }
    });
    gdt.gdt.load();

    unsafe { CS::set_reg(gdt.kernel_code_selector) };
    unsafe { SS::set_reg(gdt.kernel_data_selector) };
    unsafe { load_tss(gdt.tss_selector) };
    let idt = IDT.call_once(|| {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt[34].set_handler_fn(timer_interrupt_handler);
        idt[35].set_handler_fn(sci_interrupt_handler);
        idt[36].set_handler_fn(ehci_interrupt_handler);
        idt
    });
    idt.load();
}
