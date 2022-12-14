use futures::Stream;
use pin_project::pin_project;

use crate::{
    signal::{Signal, SignalStream},
    App,
};

use super::{stream::StreamEffect, Effect};

#[pin_project]
pub(crate) struct SignalEffect<S, F> {
    #[pin]
    inner: StreamEffect<SignalStream<S>, F>,
}

impl<S, F> SignalEffect<S, F> {
    pub fn new(signal: S, func: F) -> Self
    where
        S: for<'x> Signal<'x>,
    {
        Self {
            inner: StreamEffect::new(signal.into_stream(), func),
        }
    }
}

impl<S, T, F> Effect for SignalEffect<S, F>
where
    S: 'static + Send + Sync + Unpin + for<'x> Signal<'x, Item = T>,
    SignalStream<S>: 'static + Send + Unpin + Stream<Item = T>,
    F: 'static + Send + Sync + FnMut(&mut App, T),
{
    fn poll_effect(self: std::pin::Pin<&mut Self>, app: &mut App, cx: &mut std::task::Context<'_>) {
        self.project().inner.poll_effect(app, cx)
    }

    fn abort(&self) {
        self.inner.abort()
    }
}
