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

use crate::{acpi_events, apic::end_of_interrupt};

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
}

extern "x86-interrupt" fn timer_interrupt_handler(stack_frame: InterruptStackFrame) {
    log::info!("Timer interrupt!");
    unsafe { end_of_interrupt() };
}

extern "x86-interrupt" fn sci_interrupt_handler(stack_frame: InterruptStackFrame) {
    let events = acpi_events::pending_events();
    acpi_events::clear_events(events);
    log::info!("SCI interrupt! {events:?}.");
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
        idt
    });
    idt.load();
}
