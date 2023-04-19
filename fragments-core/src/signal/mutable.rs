use std::{
    fmt::Debug,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Weak,
    },
    task::{Context, Poll},
};

use parking_lot::{self, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::{
    waiter::{WaitList, Waiter},
    Signal,
};

pub struct Mutable<T> {
    inner: Arc<MutableInner<T>>,
}

impl<T> Mutable<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(MutableInner {
                value: RwLock::new(value),
                mutable_count: AtomicUsize::new(1),
                waiters: Default::default(),
            }),
        }
    }

    pub fn write(&self) -> MutableWriteGuard<T> {
        let value = self.inner.value.write();
        let wake_on_drop = WakeOnDrop {
            waiters: self.inner.waiters.lock(),
        };
        MutableWriteGuard {
            value,
            _wake_on_drop: wake_on_drop,
        }
    }
}

impl<T> Drop for Mutable<T> {
    fn drop(&mut self) {
        let count = self.inner.mutable_count.fetch_sub(1, Ordering::Relaxed);
        if count == 1 {
            self.inner.waiters.lock().wake_all();
        }
    }
}

impl<T> Clone for Mutable<T> {
    fn clone(&self) -> Self {
        self.inner.mutable_count.fetch_add(1, Ordering::Relaxed);
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Debug for Mutable<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mutable")
            .field("inner", &self.inner.value)
            .finish()
    }
}

struct WakeOnDrop<'a> {
    waiters: MutexGuard<'a, WaitList>,
}

impl<'a> Drop for WakeOnDrop<'a> {
    fn drop(&mut self) {
        self.waiters.wake_all()
    }
}

pub struct MutableWriteGuard<'a, T> {
    value: RwLockWriteGuard<'a, T>,
    _wake_on_drop: WakeOnDrop<'a>,
}

impl<'a, T> std::ops::Deref for MutableWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, T> std::ops::DerefMut for MutableWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

pub struct MutableReadGuard<'a, T> {
    value: RwLockReadGuard<'a, T>,
}

impl<'a, T> std::ops::Deref for MutableReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

struct MutableInner<T> {
    mutable_count: AtomicUsize,
    value: RwLock<T>,
    waiters: Mutex<WaitList>,
}

impl<T> Mutable<T> {
    pub fn signal(&self) -> MutableSignal<T> {
        let waker = Arc::new(Waiter::new(true));
        self.push_waiter(Arc::downgrade(&waker));

        MutableSignal {
            waiter: waker,
            state: Arc::downgrade(&self.inner),
        }
    }

    pub fn signal_ref(&self) -> MutableSignalRef<T> {
        let waker = Arc::new(Waiter::new(true));
        self.push_waiter(Arc::downgrade(&waker));

        MutableSignalRef {
            waker,
            state: Some(self.inner.clone()),
        }
    }

    fn push_waiter(&self, waiter: Weak<Waiter>) {
        self.inner.waiters.lock().push(waiter)
    }
}

pub struct MutableSignal<T> {
    waiter: Arc<Waiter>,
    state: Weak<MutableInner<T>>,
}

impl<'a, T> Signal<'a> for MutableSignal<T>
where
    T: 'a + Clone,
{
    type Item = T;

    fn poll_changed(self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(state) = self.state.upgrade() {
            if self.waiter.take_changed() {
                let item = state.value.read().clone();
                Poll::Ready(Some(item))
            } else {
                // Store a waker
                self.waiter.set_waker(cx.waker().clone());
                Poll::Pending
            }
        } else {
            Poll::Ready(None)
        }
    }
}

pub struct MutableSignalRef<T> {
    waker: Arc<Waiter>,
    /// Using a `Weak` here is not possible as a lock needs to be returned
    state: Option<Arc<MutableInner<T>>>,
}

impl<'a, T> Signal<'a> for MutableSignalRef<T>
where
    T: 'a,
{
    type Item = MutableReadGuard<'a, T>;

    fn poll_changed(self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // let _self = self.get_mut();
        let _self = self.get_mut();

        if let Some(state) = _self.state.as_mut() {
            if state.mutable_count.load(Ordering::Relaxed) == 0 {
                _self.state = None;
                return Poll::Ready(None);
            }
        }

        let Some(state) = _self.state.as_mut() else { return Poll::Ready(None) };

        if _self.waker.take_changed() {
            let item = MutableReadGuard {
                value: state.value.read(),
            };

            Poll::Ready(Some(item))
        } else {
            // Store a waker
            _self.waker.set_waker(cx.waker().clone());
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod test {

    use futures::FutureExt;

    use super::*;
    #[tokio::test]
    async fn mutable() {
        let value = Mutable::new("foo".to_string());

        let mut signal = value.signal();
        let mut signal_ref = value.signal_ref();

        assert_eq!(
            signal.next_value().now_or_never(),
            Some(Some("foo".to_string()))
        );

        assert_eq!(
            signal_ref.by_ref().cloned().next_value().now_or_never(),
            Some(Some("foo".to_string()))
        );

        let task = tokio::spawn(async move {
            let v = signal.next_value().await;
            assert_eq!(v, None);
        });

        tokio::task::yield_now().await;

        drop(value);

        assert_eq!(
            signal_ref.map(|v| v.clone()).next_value().now_or_never(),
            Some(None)
        );

        tokio::task::yield_now().await;

        eprintln!("Joining task");
        assert!(task.now_or_never().is_some());
    }
}
