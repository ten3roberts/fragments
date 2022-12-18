use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::Stream;
use pin_project::pin_project;

use crate::App;

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

impl<S, F> Effect for StreamEffect<S, F>
where
    S: Stream,
    F: FnMut(&mut App, S::Item),
{
    fn poll_effect(
        self: Pin<&mut Self>,
        app: &mut crate::App,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let p = self.project();
        // Project and lock
        eprintln!("Effect ready");

        let mut stream = p.stream;
        let func = p.func;

        loop {
            let Poll::Ready(item) = stream.as_mut().poll_next(cx) else {
                return Poll::Pending;
            };

            if let Some(item) = item {
                (func)(app, item)
            } else {
                break;
            }
        }

        Poll::Ready(())
    }

    type Output = ();
}
