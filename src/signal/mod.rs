mod app_signal;
mod map;
mod waiter;

pub use app_signal::*;
pub use map::*;

use std::{
    pin::Pin,
    sync::{Arc, Weak},
    task::{Context, Poll},
};

use futures::{Future, Stream};
use parking_lot::{RwLock, RwLockWriteGuard};

use self::{
    map::Map,
    waiter::{WaitList, Waiter},
};

pub trait Signal<'a> {
    type Item: 'a;
    // where
    //     Self: 'a;

    /// Polls the signal
    ///
    /// When the next item is ready, `resolve` will be used to turn the temporary borrow into an
    /// Item, E.g; by cloning.
    fn poll_changed(self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;

    fn next_value(&mut self) -> SignalFuture<&mut Self> {
        SignalFuture { signal: self }
    }

    fn map<F, U>(self, f: F) -> Map<Self, F>
    where
        F: FnMut(Self::Item) -> U,
        Self: Sized,
    {
        Map { signal: self, f }
    }

    fn by_ref(&mut self) -> &mut Self {
        self
    }

    fn into_stream(self) -> SignalStream<Self>
    where
        Self: Sized,
    {
        SignalStream { signal: self }
    }
}

impl<'a, 's, S> Signal<'a> for &'s mut S
where
    S: Unpin + Signal<'a>,
{
    type Item = S::Item;

    fn poll_changed(self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let v = &mut **self.get_mut();
        Pin::new(v).poll_changed(cx)
    }
}

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

    fn poll_changed(self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        eprintln!("Polling changed");
        if self.waker.take_changed() {
            if let Some(state) = self.state.upgrade() {
                let item = state.read().value.clone();
                eprintln!("Got item");
                Poll::Ready(Some(item))
            } else {
                eprintln!("No");
                Poll::Ready(None)
            }
        } else {
            // Store a waker
            self.waker.set_waker(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct SignalStream<S> {
    signal: S,
}

impl<S, T> Stream for SignalStream<S>
where
    S: Unpin + for<'x> Signal<'x, Item = T>,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let signal = Pin::new(&mut Pin::get_mut(self).signal);
        signal.poll_changed(cx)
    }
}

pub struct SignalFuture<S> {
    signal: S,
}

impl<S, T> Future for SignalFuture<S>
where
    S: Unpin + for<'x> Signal<'x, Item = T>,
{
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let signal = Pin::new(&mut Pin::get_mut(self).signal);
        signal.poll_changed(cx)
    }
}

#[cfg(test)]
mod test {
    use futures::FutureExt;

    use super::*;

    #[tokio::test]
    async fn mutable() {
        let value = Mutable::new(5);

        let mut s0 = value.signal();
        let mut s1 = value.signal();
        let mut s2 = value.signal();

        let task = tokio::spawn(async move {
            let value = s2.next_value().await;
            assert_eq!(value, Some(4));
        });

        assert_eq!(s0.next_value().now_or_never(), Some(Some(5)));

        assert_eq!(s0.next_value().now_or_never(), None);

        *value.write() *= 2;

        assert_eq!(s0.next_value().now_or_never(), Some(Some(10)));
        assert_eq!(s1.next_value().now_or_never(), Some(Some(10)));

        assert_eq!(s1.next_value().now_or_never(), None);
        *value.write() = 4;

        assert_eq!(s1.next_value().now_or_never(), Some(Some(4)));

        task.await.unwrap();
    }
}
