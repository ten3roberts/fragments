use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
    task::{Context, Wake, Waker as AsyncWaker},
};

use parking_lot::Mutex;

#[derive(Clone)]
pub enum SignalWaker {
    None,
    AsyncWaker(AsyncWaker),
    Callback(Arc<dyn Fn() + Send + Sync>),
}

impl SignalWaker {
    pub fn from_cx(cx: &Context<'_>) -> Self {
        Self::AsyncWaker(cx.waker().clone())
    }
}

impl Default for SignalWaker {
    fn default() -> Self {
        Self::None
    }
}

impl From<AsyncWaker> for SignalWaker {
    fn from(v: AsyncWaker) -> Self {
        Self::AsyncWaker(v)
    }
}

impl SignalWaker {
    fn wake(&self) {
        match self {
            SignalWaker::None => {}
            SignalWaker::AsyncWaker(v) => v.wake_by_ref(),
            SignalWaker::Callback(v) => v(),
        }
    }
}

pub(crate) struct Waiter {
    changed: AtomicBool,
    // Method to use to signal the change
    waker: Mutex<SignalWaker>,
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

    pub fn set_waker(&self, waker: SignalWaker) {
        *self.waker.lock() = waker;
    }

    pub fn wake(&self) {
        self.changed.store(true, Ordering::SeqCst);
        self.waker.lock().wake();
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
