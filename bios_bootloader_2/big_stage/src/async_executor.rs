use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use x86_64::instructions::interrupts;

pub fn execute_async<F: Future>(future: F) -> F::Output {
    let mut pinned = pin!(future);

    let waker = Waker::noop();
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
                // Halt just until an interrupt, without missing any interrupts
                interrupts::enable_and_hlt();
            }
        }
    }
}
