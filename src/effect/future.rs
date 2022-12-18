use std::{
    pin::Pin,
    sync::atomic::AtomicBool,
    task::{Context, Poll},
};

use futures::{Future, FutureExt};
use pin_project::pin_project;

use crate::App;

use super::{Effect, EffectSender};

/// An effect which run the future to completion, and executes the provided function when finished
#[pin_project]
pub struct FutureEffect<Fut, F>
where
    Fut: Future,
    F: FnOnce(&mut App, Fut::Output),
{
    #[pin]
    fut: Fut,
    func: Option<F>,
    queue: EffectSender,
    ready: AtomicBool,
}

impl<Fut, F> Effect for FutureEffect<Fut, F>
where
    Fut: Future,
    F: FnOnce(&mut App, Fut::Output),
{
    type Output = ();
    fn poll_effect(
        self: Pin<&mut Self>,
        app: &mut App,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        // Project and lock
        eprintln!("Effect ready");

        let p = self.project();
        let mut fut = p.fut;
        if let Poll::Ready(item) = fut.poll_unpin(cx) {
            let func = p.func.take().unwrap();
            (func)(app, item);
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
