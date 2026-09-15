use core::{
    alloc::GlobalAlloc,
    sync::atomic::{AtomicBool, Ordering},
};

use kernel_common::global_allocator::KernelGlobalAllocator;
use spin::Once;

pub struct UefiKernelGlobalAllocator {
    uefi_allocator: uefi::allocator::Allocator,
    disabled: AtomicBool,
    kernel_allocator: Once<KernelGlobalAllocator>,
}

impl UefiKernelGlobalAllocator {
    fn allocator(&self) -> &dyn GlobalAlloc {
        if self.disabled.load(Ordering::Relaxed) {
            panic!("Allocator is disabled (exited boot services and didn't init kernel yet)")
        }
        if let Some(kernel_allocator) = self.kernel_allocator.get() {
            kernel_allocator
        } else {
            &self.uefi_allocator
        }
    }
}

impl UefiKernelGlobalAllocator {
    pub const fn new() -> Self {
        Self {
            uefi_allocator: uefi::allocator::Allocator,
            disabled: AtomicBool::new(false),
            kernel_allocator: Once::new(),
        }
    }

    pub fn disable_uefi(&self) {
        self.disabled.store(true, Ordering::Relaxed);
    }

    pub fn switch_to_kernel(&self, kernel_allocator: KernelGlobalAllocator) {
        self.kernel_allocator.call_once(|| kernel_allocator);
        self.disabled.store(false, Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for UefiKernelGlobalAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        unsafe { self.allocator().alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        unsafe { self.allocator().dealloc(ptr, layout) }
    }

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: core::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        unsafe { self.allocator().realloc(ptr, layout, new_size) }
    }

    unsafe fn alloc_zeroed(&self, layout: core::alloc::Layout) -> *mut u8 {
        unsafe { self.allocator().alloc_zeroed(layout) }
    }
}

#[global_allocator]
pub static GLOBAL_ALLOCATOR: UefiKernelGlobalAllocator = UefiKernelGlobalAllocator::new();
