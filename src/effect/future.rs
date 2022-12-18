use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, FutureExt};
use pin_project::pin_project;

use super::Effect;

/// An effect which run the future to completion, and executes the provided function when finished
#[pin_project]
pub struct FutureEffect<Fut, F>
where
    Fut: Future,
{
    #[pin]
    fut: Fut,
    func: Option<F>,
}

impl<Fut, F> FutureEffect<Fut, F>
where
    Fut: Future,
{
    pub fn new(future: Fut, func: F) -> Self {
        Self {
            fut: future,
            func: Some(func),
        }
    }
}

impl<Data, Fut, F> Effect<Data> for FutureEffect<Fut, F>
where
    Fut: Future,
    F: FnOnce(&mut Data, Fut::Output),
{
    fn poll_effect(self: Pin<&mut Self>, ctx: &mut Data, async_cx: &mut Context<'_>) -> Poll<()> {
        let p = self.project();
        let mut fut = p.fut;
        if let Poll::Ready(item) = fut.poll_unpin(async_cx) {
            let func = p.func.take().unwrap();
            (func)(ctx, item);
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
