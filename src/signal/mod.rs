pub mod hold;
mod map;
mod mutable;
mod waiter;

pub use map::*;
pub use mutable::*;
use pin_project::pin_project;

use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

use futures::{ready, Future, Stream, StreamExt};

use self::{hold::Hold, map::Map};

/// A signal represents a value which can change and be observed
pub trait Signal<'a> {
    type Item: 'a;

    /// Polls the signal until the value changes
    fn poll_changed(self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;

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
        Self: Sized,
        F: for<'x> FnMut(<Self as Signal<'x>>::Item) -> U,
        U: 'a,
    {
        Map { signal: self, f }
    }

    fn by_ref(&mut self) -> &mut Self {
        self
    }

    /// Convert the values into owned values, by deref and cloning
    fn cloned(self) -> Cloned<Self>
    where
        Self: Sized,
    {
        Cloned { signal: self }
    }

    fn into_stream(self) -> SignalStream<Self>
    where
        Self: Sized,
    {
        SignalStream { signal: self }
    }
}

/// Convert a future into a signal which yields one item
pub fn from_future<F>(future: F) -> FromFuture<F> {
    FromFuture { fut: Some(future) }
}

#[pin_project]
pub struct FromFuture<F> {
    #[pin]
    fut: Option<F>,
}

impl<'a, F> Signal<'a> for FromFuture<F>
where
    F: Future,
    F::Output: 'a,
{
    type Item = F::Output;

    fn poll_changed(self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut p = self.project();
        match p.fut.as_mut().as_pin_mut() {
            Some(fut) => {
                let value = ready!(fut.poll(cx));
                p.fut.set(None);
                Poll::Ready(Some(value))
            }
            // Future has already completed
            None => Poll::Ready(None),
        }
    }
}

/// Convert a stream into a signal
pub fn from_stream<S: Stream>(stream: S) -> FromStream<S> {
    FromStream {
        stream: stream.fuse(),
    }
}

use futures::stream::Fuse;

#[pin_project]
pub struct FromStream<S> {
    #[pin]
    stream: Fuse<S>,
}

impl<'a, S> Signal<'a> for FromStream<S>
where
    S: Stream,
    S::Item: 'a,
{
    type Item = S::Item;

    fn poll_changed(self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().stream.poll_next(cx)
    }
}

impl<'a, 's, S> Signal<'a> for &'s mut S
where
    S: Unpin + Signal<'a>,
{
    type Item = S::Item;

    fn poll_changed(self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let v = self.get_mut();
        Pin::new(v.deref_mut()).poll_changed(cx)
    }
}

#[pin_project]
pub struct Cloned<S> {
    #[pin]
    signal: S,
}

impl<'a, S, U, T> Signal<'a> for Cloned<S>
where
    S: Signal<'a, Item = U>,
    U: Deref<Target = T>,
    T: 'static + Clone,
{
    type Item = T;

    fn poll_changed(self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project()
            .signal
            .poll_changed(cx)
            .map(|v| v.map(|v| v.deref().clone()))
    }
}

impl<'a, S> Signal<'a> for Box<S>
where
    S: Unpin + Signal<'a>,
{
    type Item = S::Item;

    fn poll_changed(self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let s = self.get_mut();
        Pin::new(s.deref_mut()).poll_changed(cx)
    }
}

impl<'a, P> Signal<'a> for Pin<P>
where
    P: DerefMut,
    <P as Deref>::Target: Signal<'a>,
{
    type Item = <<P as Deref>::Target as Signal<'a>>::Item;

    fn poll_changed(self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Manual implementation of `as_deref_mut`
        //
        // See: https://github.com/rust-lang/rust/issues/86918
        unsafe { self.get_unchecked_mut() }
            .as_mut()
            .poll_changed(cx)
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

#[pin_project]
pub struct SignalFuture<S> {
    #[pin]
    signal: S,
}

impl<S, T> Future for SignalFuture<S>
where
    S: for<'x> Signal<'x, Item = T>,
{
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let p = self.project();
        p.signal.poll_changed(cx)
    }
}

#[cfg(test)]
mod test {

    use std::time::Duration;

    use futures::FutureExt;
    use tokio::time::sleep;

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

    #[tokio::test]
    async fn from_future() {
        let future = Box::pin(async {
            sleep(Duration::from_secs(1)).await;

            "Hello, World!"
        });

        let mut signal = super::from_future(future);

        assert_eq!(signal.next_value().await, Some("Hello, World!"))
    }
}
