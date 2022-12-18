use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::Stream;
use pin_project::pin_project;

use super::Effect;

/// An effect which executes the provided function for each item in the stream
#[pin_project]
pub struct StreamEffect<S, F> {
    #[pin]
    stream: S,
    func: F,
}

impl<S, F> StreamEffect<S, F> {
    pub fn new(stream: S, func: F) -> Self {
        Self { stream, func }
    }
}

impl<Data, S, F> Effect<Data> for StreamEffect<S, F>
where
    S: Stream,
    F: FnMut(&mut Data, S::Item),
{
    fn poll_effect(self: Pin<&mut Self>, ctx: &mut Data, async_cx: &mut Context<'_>) -> Poll<()> {
        let p = self.project();
        // Project and lock
        let mut stream = p.stream;
        let func = p.func;

        loop {
            let Poll::Ready(item) = stream.as_mut().poll_next(async_cx) else {
                return Poll::Pending;
            };

            if let Some(item) = item {
                (func)(ctx, item)
            } else {
                break;
            }
        }

        Poll::Ready(())
    }
}
