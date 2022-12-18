use std::task::Poll;

use pin_project::pin_project;

use crate::{signal::Signal, App};

use super::Effect;

#[pin_project]
pub(crate) struct SignalEffect<S, F> {
    #[pin]
    signal: S,
    func: F,
}

impl<S, F> SignalEffect<S, F> {
    pub fn new(signal: S, func: F) -> Self
    where
        S: for<'x> Signal<'x>,
    {
        Self { signal, func }
    }
}

impl<S, T, F> Effect for SignalEffect<S, F>
where
    S: for<'x> Signal<'x, Item = T>,
    F: FnMut(&mut App, T),
{
    fn poll_effect(
        self: std::pin::Pin<&mut Self>,
        app: &mut App,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let p = self.project();
        // Project and lock
        eprintln!("Effect ready");

        let mut signal = p.signal;
        let func = p.func;
        loop {
            let Poll::Ready(item) = signal.as_mut().poll_changed(cx) else {
                return Poll::Pending
            };

            if let Some(item) = item {
                (func)(app, item);
            } else {
                break;
            }
        }

        // Done
        Poll::Ready(())
    }

    type Output = ();
}
