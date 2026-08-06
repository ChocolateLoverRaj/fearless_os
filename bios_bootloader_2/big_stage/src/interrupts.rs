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
static GDT_RAW: GlobalDescriptorTable = GlobalDescriptorTable::from_raw_entries(&[
    // Null segment (required)
    0x0000000000000000,
    // Code 64
    0x00209A0000000000,
    // Code 32
    0x00CF9A000000FFFF,
    // Code 16
    0x000F9A000000FFFF,
    // Data 64
    0x0000920000000000,
    // Data 32
    0x00CF92000000FFFF,
    // Data 16
    0x000092000000FFFF,
]);

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
    // GDT_RAW.load();
    //
    // loop {}
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
