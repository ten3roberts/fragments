mod executor;
mod future;
mod signal;
mod stream;

pub use executor::*;
pub use future::*;
use futures::Stream;
pub(crate) use signal::*;
pub use stream::*;

use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Represents an asynchronous computation which has access some short lived context when polling.
///
/// This can be the app as a whole, or a scope.
pub trait Effect<Data> {
    fn poll_effect(self: Pin<&mut Self>, data: &mut Data, async_cx: &mut Context<'_>) -> Poll<()>;
}

/// Convert any [`futures::Stream`] into an effect which executes for each item
pub fn from_stream<Data, S, F>(stream: S, func: F) -> StreamEffect<S, F>
where
    S: Stream,
    F: FnMut(&mut Data, S::Item),
{
    StreamEffect::new(stream, func)
}

pub struct FnOnceEffect<F> {
    func: Option<F>,
}

impl<F> FnOnceEffect<F> {
    pub fn new(func: F) -> Self {
        Self { func: Some(func) }
    }
}

impl<Data, F> Effect<Data> for FnOnceEffect<F>
where
    F: Unpin + FnOnce(&mut Data),
{
    fn poll_effect(mut self: Pin<&mut Self>, data: &mut Data, _: &mut Context<'_>) -> Poll<()> {
        (self.func.take().unwrap())(data);

        Poll::Ready(())
    }
}

// /// Various ways of turning *something* into an effect using the supplied applicative.
// pub trait IntoEffect<Data, A> {
//     type Effect;
//     fn into_effect(self, apply: A) -> Self::Effect;
// }

// pub trait StreamExt: Stream + Sized {
//     fn into_effect<F>(self, func: F) -> StreamEffect<Self, F>;
// }

// impl<S> StreamExt for S
// where
//     S: Stream + Sized,
// {
//     fn into_effect<F>(self, func: F) -> StreamEffect<Self, F> {
//         StreamEffect::new(self, func)
//     }
// }
