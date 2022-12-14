use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Stream, StreamExt};
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
    S: 'static + Send + Sync + Stream,
    F: 'static + Send + Sync + FnMut(&mut App, S::Item),
{
    fn poll_effect(self: Pin<&mut Self>, app: &mut crate::App, cx: &mut Context<'_>) {
        let p = self.project();
        // Project and lock
        eprintln!("Effect ready");

        let mut stream = p.stream;
        let func = p.func;
        while let Poll::Ready(Some(item)) = stream.poll_next_unpin(cx) {
            (func)(app, item)
        }
    }

    fn abort(&self) {
        todo!()
    }
}
