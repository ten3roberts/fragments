use std::{
    pin::Pin,
    sync::{Arc, Weak},
    task::Poll,
};

use parking_lot::{RwLock, RwLockWriteGuard};

use super::{
    waiter::{SignalWaker, WaitList, Waiter},
    Signal,
};

pub struct Mutable<T> {
    inner: Arc<RwLock<MutableInner<T>>>,
}

impl<T> Mutable<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(MutableInner {
                value,
                wakers: Default::default(),
            })),
        }
    }

    pub fn write(&self) -> MutableWriteGuard<T> {
        let inner = self.inner.write();
        MutableWriteGuard { inner }
    }
}

pub struct MutableWriteGuard<'a, T> {
    inner: RwLockWriteGuard<'a, MutableInner<T>>,
}

impl<'a, T> std::ops::Deref for MutableWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

impl<'a, T> std::ops::DerefMut for MutableWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner.value
    }
}

impl<'a, T> Drop for MutableWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.inner.wakers.wake_all();
    }
}

struct MutableInner<T> {
    value: T,
    wakers: WaitList,
}

impl<T> Mutable<T> {
    pub fn signal(&self) -> MutableSignal<T> {
        let waker = Arc::new(Waiter::new(true));
        let mut inner = self.inner.write();
        inner.wakers.push(Arc::downgrade(&waker));

        MutableSignal {
            waker,
            state: Arc::downgrade(&self.inner),
        }
    }
}

pub struct MutableSignal<T> {
    waker: Arc<Waiter>,
    state: Weak<RwLock<MutableInner<T>>>,
}

impl<'a, T> Signal<'a> for MutableSignal<T>
where
    T: 'a + Clone,
{
    type Item = T;

    fn poll_changed(self: Pin<&'a mut Self>, waker: SignalWaker) -> Poll<Option<Self::Item>> {
        eprintln!("Polling changed");
        if let Some(state) = self.state.upgrade() {
            if self.waker.take_changed() {
                let item = state.read().value.clone();
                eprintln!("Got item");
                Poll::Ready(Some(item))
            } else {
                // Store a waker
                self.waker.set_waker(waker.clone());
                Poll::Pending
            }
        } else {
            eprintln!("No");
            Poll::Ready(None)
        }
    }
}
