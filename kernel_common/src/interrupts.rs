use core::str::FromStr;

use acpi::{
    aml::{
        AmlError,
        namespace::AmlName,
        object::{Object, WrappedObject},
    },
    registers::Pm1EventFlags,
};
use alloc::vec;
use log::logger;
use spin::{Mutex, Once};
use x86_64::{
    instructions::tables::load_tss,
    registers::segmentation::{CS, SS, Segment},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        idt::{HandlerFuncType, InterruptDescriptorTable, InterruptStackFrame},
        tss::TaskStateSegment,
    },
};

use crate::{
    acpi_events::{self, ACPI_GLOBALS, platform},
    apic::end_of_interrupt,
};

pub struct Gdt {
    gdt: GlobalDescriptorTable<5>,
    kernel_code_selector: SegmentSelector,
    kernel_data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

struct IdtInfo {
    idt: InterruptDescriptorTable,
    irqs_assigned: u8,
}

static IDT: Mutex<IdtInfo> = Mutex::new(IdtInfo {
    idt: InterruptDescriptorTable::new(),
    irqs_assigned: 0,
});
static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<Gdt> = Once::new();

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IrqAssignments {
    LapicError = 0x20,
    LapicSpurious,
    LapicTimer,
    Sci,
}

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
            .set_sleep_typ((*slp_type_a).try_into().unwrap())
            .unwrap();
        platform
            .registers
            .pm1_control_registers
            .set_bit(acpi::registers::Pm1ControlBit::SleepEnable, true)
            .unwrap();
        log::info!("Did shutdown. You shouldn't see this");
        logger().flush();
    }
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
    let mut idt = IDT.lock();
    idt.idt.breakpoint.set_handler_fn(breakpoint_handler);
    idt.idt[IrqAssignments::LapicTimer as u8].set_handler_fn(timer_interrupt_handler);
    idt.idt[IrqAssignments::Sci as u8].set_handler_fn(sci_interrupt_handler);
    idt.irqs_assigned = IrqAssignments::Sci as u8;
    // idt[IrqAssignments::Ehci as u8].set_handler_fn(ehci_interrupt_handler);
    // idt[IrqAssignments::Hpet as u8].set_handler_fn(hpet_interrupt_handler);
    unsafe { idt.idt.load_unsafe() };
}

pub fn assign_irq(f: extern "x86-interrupt" fn(InterruptStackFrame)) -> u8 {
    let mut idt = IDT.lock();
    let irq = idt
        .irqs_assigned
        .checked_add(1)
        .expect("out of IRQ numbers");
    idt.idt[irq].set_handler_fn(f);
    idt.irqs_assigned += 1;
    irq
}
