use core::future::Future;
use core::pin::pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};

use alloc::sync::Arc;
use alloc::task::Wake;
use x86_64::instructions::interrupts;

#[derive(Debug, Default)]
struct BooleanWaker {
    woken: AtomicBool,
}

impl BooleanWaker {
    pub fn take(&self) -> bool {
        self.woken.swap(false, Ordering::Relaxed)
    }

    pub fn woken(&self) -> bool {
        self.woken.load(Ordering::Relaxed)
    }

    pub fn mark_not_woken(&self) {
        self.woken.store(false, Ordering::Relaxed);
    }
}

impl Wake for BooleanWaker {
    fn wake(self: alloc::sync::Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &alloc::sync::Arc<Self>) {
        self.woken.store(true, Ordering::Relaxed);
    }
}

pub fn execute_future<F: Future>(future: F) -> F::Output {
    let mut pinned = pin!(future);

    // FIXME: Avoid constant polling by only re-polling if waker was woken, not after each hlt ended
    let boolean_waker = Arc::new(BooleanWaker::default());
    let waker = Waker::from(boolean_waker.clone());
    let mut cx = Context::from_waker(&waker);
    loop {
        // Disable interrupts so that we don't go to sleep just after receiving an interrupt
        interrupts::disable();
        match pinned.as_mut().poll(&mut cx) {
            Poll::Ready(output) => {
                interrupts::enable();
                break output;
            }
            Poll::Pending => {
                loop {
                    // Halt just until an interrupt, without missing any interrupts
                    interrupts::enable_and_hlt();
                    if boolean_waker.take() {
                        break;
                    }
                }
            }
        }
    }
}
