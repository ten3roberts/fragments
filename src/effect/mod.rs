mod executor;
mod future;
mod signal;
mod stream;

pub(crate) use executor::*;
pub use future::*;
use futures::{Future, Stream, StreamExt};
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
