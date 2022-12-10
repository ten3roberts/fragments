use std::{
    pin::Pin,
    sync::{atomic::AtomicBool, Arc},
    task::Poll,
};

use super::{waiter::SignalWaker, Signal};

#[derive(Debug, Clone)]
pub struct Hold<S, T> {
    signal: S,
    value: Option<T>,
    is_closed: bool,
}

impl<S, T> Hold<S, T>
where
    T: 'static,
{
    pub fn new(signal: S) -> Self {
        Self {
            signal,
            value: None,
            is_closed: false,
        }
    }

    /// Returns the most recent value of the signal.
    ///
    /// Returns None if there has not yet been a value for the signal or the signal is closed.
    pub fn get(&mut self) -> Option<&T>
    where
        S: Unpin + for<'x> Signal<'x, Item = T>,
    {
        let signal = Pin::new(&mut self.signal);

        match signal.poll_changed(SignalWaker::None) {
            Poll::Ready(Some(v)) => self.value = Some(v),
            Poll::Ready(None) => {
                self.is_closed = true;
                return None;
            }
            Poll::Pending => {}
        }

        self.value.as_ref()
    }

    pub fn is_closed(&self) -> bool {
        self.is_closed
    }
}

#[cfg(test)]
mod test {
    use crate::signal::Mutable;

    use super::*;

    #[test]
    fn hold() {
        let value = Mutable::new("Hello, World!".to_string());

        let signal = value.signal().map(|v| v.to_lowercase());

        let mut sink = signal.hold();

        assert_eq!(sink.get(), Some(&"hello, world!".to_string()));
        assert_eq!(sink.get(), Some(&"hello, world!".to_string()));
        *value.write() = "Foo".to_string();

        assert_eq!(sink.get(), Some(&"foo".to_string()));
        drop(value);
        assert_eq!(sink.get(), None);
    }
}
