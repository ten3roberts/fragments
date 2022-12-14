use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Stream, StreamExt};
use parking_lot::Mutex;

use crate::App;

use super::Effect;

/// An effect which executes the provided function for each item in the stream
pub struct StreamEffect<S, F> {
    stream: Mutex<Option<S>>,
    func: Mutex<F>,
}

impl<S, F> StreamEffect<S, F> {
    pub fn new(stream: S, func: F) -> Self {
        Self {
            stream: Mutex::new(Some(stream)),
            func: Mutex::new(func),
        }
    }
}

impl<S, F> Effect for StreamEffect<S, F>
where
    S: 'static + Send + Unpin + Stream,
    F: 'static + Send + FnMut(&mut App, S::Item),
{
    fn poll_effect(self: Pin<&mut Self>, app: &mut crate::App, cx: &mut Context<'_>) {
        // Project and lock
        eprintln!("Effect ready");

        let mut stream = self.stream.lock();
        if let Some(stream) = &mut *stream {
            let mut func = self.func.lock();
            while let Poll::Ready(Some(item)) = stream.poll_next_unpin(cx) {
                (func)(app, item)
            }
        }
    }

    fn abort(&self) {
        *self.stream.lock() = None
    }
}
