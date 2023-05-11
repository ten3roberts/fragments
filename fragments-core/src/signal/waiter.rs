use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Weak,
    },
    task::{Context, Poll, Waker as AsyncWaker},
};

use parking_lot::Mutex;

/// Abstraction over the most recent async waker and changed flag
pub(crate) struct Hook {
    changed: AtomicBool,
    // Method to use to signal the change
    waker: Mutex<Option<AsyncWaker>>,
}

impl Hook {
    pub fn new(initial_changed: bool) -> Self {
        Self {
            changed: AtomicBool::new(initial_changed),
            waker: Default::default(),
        }
    }
}

impl Hook {
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
pub(crate) struct HookList {
    inner: Mutex<Vec<Weak<Hook>>>,
}

impl HookList {
    pub fn push(&self, value: Weak<Hook>) {
        let mut inner = self.inner.lock();

        inner.push(value);
        inner.retain(|v| v.strong_count() > 0);
    }

    /// Invokes all hooks
    pub fn wake_all(&self) {
        self.inner.lock().retain(|v| {
            if let Some(w) = v.upgrade() {
                w.wake();
                true
            } else {
                false
            }
        })
    }
}
