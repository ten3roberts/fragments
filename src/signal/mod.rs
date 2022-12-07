mod app_signal;
pub mod hold;
mod map;
mod mutable;
mod waiter;

pub use app_signal::*;
pub use map::*;
pub use mutable::*;

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, Stream};

use self::{hold::Hold, map::Map, waiter::SignalWaker};

pub trait Signal<'a> {
    type Item: 'a;
    // where
    //     Self: 'a;

    /// Polls the signal until the value changes
    fn poll_changed(self: Pin<&'a mut Self>, waker: SignalWaker) -> Poll<Option<Self::Item>>;

    fn next_value(&mut self) -> SignalFuture<&mut Self> {
        SignalFuture { signal: self }
    }

    fn hold(self) -> Hold<Self, Self::Item>
    where
        Self: Sized + Unpin,
        Self::Item: 'static,
    {
        Hold::new(self)
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

    fn poll_changed(self: Pin<&'a mut Self>, cx: SignalWaker) -> Poll<Option<Self::Item>> {
        let v = &mut **self.get_mut();
        Pin::new(v).poll_changed(cx)
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
        signal.poll_changed(SignalWaker::from_cx(cx))
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
        signal.poll_changed(SignalWaker::from_cx(cx))
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
