use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use x86_64::instructions::interrupts;

pub fn execute_future<F: Future>(future: F) -> F::Output {
    log::trace!("executing future");
    let mut pinned = pin!(future);

    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);

    loop {
        // Disable interrupts so that we don't go to sleep just after receiving an interrupt
        interrupts::disable();
        log::trace!("calling poll");
        match pinned.as_mut().poll(&mut cx) {
            Poll::Ready(output) => {
                interrupts::enable();
                break output;
            }
            Poll::Pending => {
                log::trace!("pending. enabling interrupts and halting.");
                // Halt just until an interrupt, without missing any interrupts
                interrupts::enable_and_hlt();
            }
        }
    }
}
