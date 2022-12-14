use std::{
    pin::Pin,
    sync::atomic::AtomicBool,
    task::{Context, Poll},
};

use futures::{Future, FutureExt};
use parking_lot::Mutex;

use crate::App;

use super::{Effect, EffectSender};

/// An effect which run the future to completion, and executes the provided function when finished
pub struct FutureEffect<Fut, F>
where
    Fut: Future,
    F: FnOnce(&mut App, Fut::Output),
{
    fut: Mutex<Option<Fut>>,
    func: Mutex<Option<F>>,
    queue: EffectSender,
    ready: AtomicBool,
}

impl<Fut, F> Effect for FutureEffect<Fut, F>
where
    Fut: 'static + Send + Unpin + Future,
    F: 'static + Send + FnOnce(&mut App, Fut::Output),
{
    fn poll_effect(self: Pin<&mut Self>, app: &mut App, cx: &mut Context<'_>) {
        // Project and lock
        eprintln!("Effect ready");

        let mut fut = self.fut.lock();
        if let Some(fut) = &mut *fut {
            while let Poll::Ready(item) = fut.poll_unpin(cx) {
                let func = self.func.lock().take().unwrap();
                (func)(app, item)
            }
        }
    }

    fn abort(&self) {
        *self.fut.lock() = None
    }
}
