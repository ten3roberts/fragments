use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Weak,
    },
    task::Waker as AsyncWaker,
};

use parking_lot::Mutex;

pub(crate) struct Waiter {
    changed: AtomicBool,
    // Method to use to signal the change
    waker: Mutex<Option<AsyncWaker>>,
}

impl Waiter {
    pub fn new(initial_changed: bool) -> Self {
        Self {
            changed: AtomicBool::new(initial_changed),
            waker: Default::default(),
        }
    }
}

impl Waiter {
    pub fn take_changed(&self) -> bool {
        self.changed
            .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    pub fn set_waker(&self, waker: AsyncWaker) {
        *self.waker.lock() = Some(waker);
    }

    pub fn wake(&self) {
        self.changed.store(true, Ordering::SeqCst);
        if let Some(waker) = &*self.waker.lock() {
            waker.wake_by_ref()
        }
    }
}

#[derive(Default)]
pub(crate) struct WaitList {
    inner: Vec<Weak<Waiter>>,
}

impl WaitList {
    pub fn push(&mut self, value: Weak<Waiter>) {
        self.inner.push(value)
    }

    pub fn wake_all(&mut self) {
        self.inner.retain(|v| {
            if let Some(w) = v.upgrade() {
                w.wake();
                true
            } else {
                false
            }
        })
    }
}
