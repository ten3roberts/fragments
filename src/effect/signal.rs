use std::task::Poll;

use pin_project::pin_project;

use crate::signal::Signal;

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

impl<Data, S, T, F> Effect<Data> for SignalEffect<S, F>
where
    S: for<'x> Signal<'x, Item = T>,
    F: FnMut(&mut Data, T),
{
    fn poll_effect<'d>(
        self: std::pin::Pin<&mut Self>,
        ctx: &mut Data,
        async_cx: &mut std::task::Context<'_>,
    ) -> Poll<()> {
        let p = self.project();
        // Project and lock

        let mut signal = p.signal;
        let func = p.func;
        loop {
            let Poll::Ready(item) = signal.as_mut().poll_changed(async_cx) else {
                return Poll::Pending
            };

            if let Some(item) = item {
                (func)(ctx, item);
            } else {
                break;
            }
        }

        // Done
        Poll::Ready(())
    }
}
