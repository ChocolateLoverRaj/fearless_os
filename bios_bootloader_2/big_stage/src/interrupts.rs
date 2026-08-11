use spin::Once;
use x86_64::{
    VirtAddr,
    instructions::tables::load_tss,
    registers::segmentation::{CS, SS, Segment},
    structures::{
        gdt::{Descriptor, DescriptorFlags, GlobalDescriptorTable, SegmentSelector},
        idt::{InterruptDescriptorTable, InterruptStackFrame},
        tss::TaskStateSegment,
    },
};

pub struct Gdt {
    gdt: GlobalDescriptorTable<9>,
    kernel_code_selector: SegmentSelector,
    kernel_data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

static IDT: Once<InterruptDescriptorTable> = Once::new();
static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<Gdt> = Once::new();

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    log::info!("Breakpoint! Stack frame: {stack_frame:#?}");
}

pub fn init() {
    let tss = TSS.call_once(TaskStateSegment::new);
    let gdt = GDT.call_once(|| {
        let mut gdt = GlobalDescriptorTable::<9>::empty();
        let kernel_code_selector = gdt.append(Descriptor::kernel_code_segment());
        // let kernel_code_32 = gdt.append(Descriptor::UserSegment(
        //     DescriptorFlags::KERNEL_CODE32.bits(),
        // ));
        let code_32 = gdt.append(Descriptor::UserSegment(0x00CF9A000000FFFF));
        let code_16 = gdt.append(Descriptor::UserSegment(0x000F9A000000FFFF));
        let kernel_data_selector = gdt.append(Descriptor::kernel_data_segment());
        let kernel_data_32 = gdt.append(Descriptor::UserSegment(0x00CF92000000FFFF));
        let data_16 = gdt.append(Descriptor::UserSegment(0x000092000000FFFF));
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
        idt
    });
    idt.load();
}
