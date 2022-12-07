use std::task::Poll;

use super::{waiter::SignalWaker, Signal};
use pin_project::pin_project;

#[pin_project]
pub struct Map<S, F> {
    #[pin]
    pub(crate) signal: S,
    pub(crate) f: F,
}

impl<'a, S, F, U> Signal<'a> for Map<S, F>
where
    S: Signal<'a>,
    for<'x> <S as Signal<'a>>::Item: std::fmt::Debug,
    F: FnMut(S::Item) -> U,
    U: 'a,
{
    type Item = U;

    fn poll_changed(
        self: std::pin::Pin<&'a mut Self>,
        cx: SignalWaker,
    ) -> Poll<Option<Self::Item>> {
        let p = self.project();
        match dbg!(p.signal.poll_changed(cx)) {
            Poll::Ready(Some(v)) => Poll::Ready(Some((p.f)(v))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod test {
    use futures::FutureExt;

    use crate::signal::Mutable;

    use super::*;

    #[tokio::test]
    async fn mutable() {
        let value = Mutable::new(5);

        let mut s0 = value.signal().map(|v| v.to_string());
        assert_eq!(s0.next_value().now_or_never(), Some(Some("5".to_string())));
        assert_eq!(s0.next_value().now_or_never(), None);
        *value.write() = 7;

        assert_eq!(s0.next_value().now_or_never(), Some(Some("7".to_string())));
        drop(value);

        assert_eq!(s0.next_value().now_or_never(), Some(None));
    }
}
