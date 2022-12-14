use std::task::{Context, Poll};

use super::Signal;
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
    F: FnMut(S::Item) -> U,
    U: 'a,
{
    type Item = U;

    fn poll_changed(
        self: std::pin::Pin<&'a mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let p = self.project();
        match p.signal.poll_changed(cx) {
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

    #[test]
    fn mapped() {
        let value = Mutable::new(5);

        let mut s0 = value.signal().map(|v| v.to_string());
        assert_eq!(
            s0.by_ref().next_value().now_or_never(),
            Some(Some("5".to_string()))
        );
        assert_eq!(s0.by_ref().next_value().now_or_never(), None);
        *value.write() = 7;

        assert_eq!(
            s0.by_ref().next_value().now_or_never(),
            Some(Some("7".to_string()))
        );
        drop(value);

        assert_eq!(s0.by_ref().next_value().now_or_never(), Some(None));
    }

    #[test]
    fn mapped_ref() {
        let value = Mutable::new(5);

        let mut s0 = value.signal_ref().map(|v| v.to_string());

        assert_eq!(s0.next_value().now_or_never(), Some(Some("5".to_string())));
        assert_eq!(s0.next_value().now_or_never(), None);
        *value.write() = 7;

        assert_eq!(s0.next_value().now_or_never(), Some(Some("7".to_string())));
        drop(value);

        assert_eq!(s0.next_value().now_or_never(), Some(None));
    }
}
