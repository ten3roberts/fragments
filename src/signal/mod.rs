mod waiter;

use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{Future, Stream};
use parking_lot::{RwLock, RwLockWriteGuard};

use self::waiter::{WaitList, Waiter};

pub trait Signal {
    type Item;
    // where
    //     Self: 'a;

    /// Polls the signal
    ///
    /// When the next item is ready, `resolve` will be used to turn the temporary borrow into an
    /// Item, E.g; by cloning.
    fn poll_changed<U, F: for<'x> Fn(&'x Self::Item) -> U>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        resolve: F,
    ) -> Poll<U>;

    fn next_value(&mut self) -> SignalFuture<&mut Self> {
        SignalFuture { signal: self }
    }

    fn into_stream(self) -> SignalStream<Self>
    where
        Self: Sized,
    {
        SignalStream { signal: self }
    }
}

impl<'s, S> Signal for &'s mut S
where
    S: Unpin + Signal,
{
    type Item = S::Item;

    fn poll_changed<U, F: for<'x> Fn(&'x Self::Item) -> U>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        resolve: F,
    ) -> Poll<U> {
        let v = &mut **self.get_mut();
        Pin::new(v).poll_changed(cx, resolve)
    }
}

struct Mutable<T> {
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

    pub fn write(&self) -> MutableGuard<T> {
        let inner = self.inner.write();
        MutableGuard { inner }
    }
}

struct MutableGuard<'a, T> {
    inner: RwLockWriteGuard<'a, MutableInner<T>>,
}

impl<'a, T> std::ops::Deref for MutableGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

impl<'a, T> std::ops::DerefMut for MutableGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner.value
    }
}

impl<'a, T> Drop for MutableGuard<'a, T> {
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
            state: self.inner.clone(),
        }
    }
}

struct MutableSignal<T> {
    waker: Arc<Waiter>,
    state: Arc<RwLock<MutableInner<T>>>,
}

impl<T> Signal for MutableSignal<T>
where
    T: 'static + Clone,
{
    type Item = T;

    fn poll_changed<U, F: for<'x> Fn(&'x Self::Item) -> U>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        resolve: F,
    ) -> Poll<U> {
        if self.waker.take_changed() {
            let guard = self.state.read();
            let item = resolve(&guard.value);
            Poll::Ready(item)
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

impl<S> Stream for SignalFuture<S>
where
    S: Unpin + Signal,
    S::Item: Clone,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let signal = Pin::new(&mut Pin::get_mut(self).signal);
        signal
            .poll_changed(cx.into(), |v| v.clone())
            .map(|v| v.into())
    }
}

pub struct SignalFuture<S> {
    signal: S,
}

impl<S> Future for SignalFuture<S>
where
    S: Unpin + Signal,
    S::Item: Clone,
{
    type Output = S::Item;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let signal = Pin::new(&mut Pin::get_mut(self).signal);
        signal.poll_changed(cx.into(), |v| v.clone())
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
            assert_eq!(value, 4);
        });

        assert_eq!(s0.next_value().now_or_never(), Some(5));

        assert_eq!(s0.next_value().now_or_never(), None);

        *value.write() *= 2;

        assert_eq!(s0.next_value().now_or_never(), Some(10));
        assert_eq!(s1.next_value().now_or_never(), Some(10));

        assert_eq!(s1.next_value().now_or_never(), None);
        *value.write() = 4;

        assert_eq!(s1.next_value().now_or_never(), Some(4));

        task.await.unwrap();
    }
}
