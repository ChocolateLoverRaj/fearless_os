use core::{
    array,
    ops::DerefMut,
    ptr::{NonNull, null_mut},
    str::FromStr,
    sync::atomic::{AtomicPtr, AtomicU8, Ordering},
};

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

static DYANAMIC_IRQ_HANDLERS: [AtomicPtr<()>; 256] = [const { AtomicPtr::new(null_mut()) }; _];

macro_rules! make_irq_handler {
    ($fn_name:ident, $vector:expr) => {
        extern "x86-interrupt" fn $fn_name(stack_frame: InterruptStackFrame) {
            let vector: u8 = $vector;
            let handler = NonNull::new(
                DYANAMIC_IRQ_HANDLERS[usize::try_from(vector).unwrap()].load(Ordering::Relaxed),
            )
            .expect("irq fired when no handler registered for it");

            let handler: fn(&InterruptStackFrame, u8) =
                unsafe { core::mem::transmute(handler.as_ptr()) };

            handler(&stack_frame, vector);
        }
    };
}

macro_rules! generate_256_irq_thunks {
    // 1. Terminal case: Output the static array of function pointers
    (@table $($vec:expr),*) => {
        pub static IRQ_THUNK_TABLE: [extern "x86-interrupt" fn(InterruptStackFrame); 256] = [
            $(
                {
                    extern "x86-interrupt" fn thunk(stack_frame: InterruptStackFrame) {
                        let vector: u8 = $vec;
                        let handler = NonNull::new(
                            DYANAMIC_IRQ_HANDLERS[usize::try_from(vector).unwrap()].load(Ordering::Relaxed),
                        )
                        .expect("irq fired when no handler registered for it");

                        let handler: fn(&InterruptStackFrame, u8) =
                            unsafe { core::mem::transmute(handler.as_ptr()) };

                        handler(&stack_frame, vector);
                    }
                    thunk
                }
            ),*
        ];
    };

    // 2. Helper macro pattern generators for number ranges
    (@count_256) => {
        generate_256_irq_thunks!(@table
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
            32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
            64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79,
            80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95,
            96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
            112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127,
            128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143,
            144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159,
            160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175,
            176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191,
            192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207,
            208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223,
            224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239,
            240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255
        );
    };

    // Main entry point
    () => {
        generate_256_irq_thunks!(@count_256);
    };
}

generate_256_irq_thunks!();

static LAST_USED_IDT_VECTOR: AtomicU8 = AtomicU8::new(0);

static IDT: Mutex<InterruptDescriptorTable> = Mutex::new(InterruptDescriptorTable::new());
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
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    idt[IrqAssignments::LapicTimer as u8].set_handler_fn(timer_interrupt_handler);
    idt[IrqAssignments::Sci as u8].set_handler_fn(sci_interrupt_handler);
    LAST_USED_IDT_VECTOR.store(IrqAssignments::Sci as u8, Ordering::Relaxed);
    for vector in IrqAssignments::Sci as u8 + 1..=255 {
        idt[vector].set_handler_fn(IRQ_THUNK_TABLE[usize::try_from(vector).unwrap()]);
    }
    unsafe { idt.load_unsafe() };
}

pub fn assign_irq(f: fn(&InterruptStackFrame, vector: u8)) -> u8 {
    let vector = loop {
        let last_used_vector = LAST_USED_IDT_VECTOR.load(Ordering::Relaxed);
        if last_used_vector == 255 {
            panic!("no more slots left")
        }
        let vector_to_try = last_used_vector + 1;
        if LAST_USED_IDT_VECTOR
            .compare_exchange(
                last_used_vector,
                vector_to_try,
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            break vector_to_try;
        }
    };
    DYANAMIC_IRQ_HANDLERS[usize::try_from(vector).unwrap()]
        .compare_exchange(
            null_mut(),
            f as *mut (),
            Ordering::Relaxed,
            Ordering::Relaxed,
        )
        .unwrap();
    log::info!("assigned IRQ {vector:#X}");
    vector
}
