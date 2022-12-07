use super::{waiter::SignalWaker, Signal};

pub struct Hold<S, T> {
    signal: S,
    value: T,
}

impl<'a, S, T> Signal<'a> for Hold<S, T>
where
    S: Signal<'a, Item = T>,
    T: 'a,
{
    type Item = T;

    fn poll_changed(
        self: std::pin::Pin<&'a mut Self>,
        waker: SignalWaker,
    ) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}
