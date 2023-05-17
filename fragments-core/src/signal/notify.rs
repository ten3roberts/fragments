use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Weak,
    },
    task::{Context, Poll},
};

use parking_lot::Mutex;

use super::{
    waiter::{Hook, HookList},
    Signal,
};

struct Inner<T> {
    value: Mutex<Option<T>>,
    senders: AtomicUsize,
    waiters: Mutex<Vec<Arc<Hook>>>,
}

pub struct Sender<T> {
    inner: Weak<Inner<T>>,
}

impl<T> Sender<T> {
    /// Send a value down the channel
    pub fn send(&self, value: T) {
        if let Some(inner) = self.inner.upgrade() {
            let mut guard = inner.value.lock();
            *guard = Some(value);
            inner.waiters.lock().drain(..).for_each(|v| v.wake())
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.upgrade() {
            if inner.senders.fetch_sub(1, Ordering::Relaxed) == 1 {
                inner.waiters.lock().drain(..).for_each(|v| v.wake());
            }
        }
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        if let Some(inner) = self.inner.upgrade() {
            inner.senders.fetch_add(1, Ordering::Relaxed);
        }

        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> std::fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Sender").finish()
    }
}

pub struct Receiver<T> {
    waiter: Option<Arc<Hook>>,
    inner: Arc<Inner<T>>,
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Self {
            waiter: None,
            inner: self.inner.clone(),
        }
    }
}

impl<'a, T> Signal<'a> for Receiver<T>
where
    T: 'a,
{
    type Item = T;

    fn poll_changed(mut self: Pin<&'a mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        let waiter = &mut this.waiter;
        let inner = &this.inner;
        if let Some(waiter) = waiter {
            if waiter.take_changed() {
                let item = inner.value.lock().take();
                eprintln!("Senders: {}", inner.senders.load(Ordering::Relaxed));
                if let Some(item) = item {
                    Poll::Ready(Some(item))
                } else if inner.senders.load(Ordering::Relaxed) == 0 {
                    eprintln!("Disconnected");
                    Poll::Ready(None)
                } else {
                    // Someone else took the value before us, store the waker into the queue once more
                    eprintln!("No item available, storing waker");
                    inner.waiters.lock().push(waiter.clone());
                    Poll::Pending
                }
            } else if inner.senders.load(Ordering::Relaxed) == 0 {
                eprintln!("Disconnected");
                Poll::Ready(None)
            } else {
                // Store a waker
                waiter.set_waker(cx.waker().clone());
                Poll::Pending
            }
        } else if let Some(item) = inner.value.lock().take() {
            tracing::info!("An item was available immediately");
            Poll::Ready(Some(item))
        } else if inner.senders.load(Ordering::Relaxed) == 0 {
            tracing::info!("Disconnected");
            Poll::Ready(None)
        } else {
            let waiter = Arc::new(Hook::new(false));
            inner.waiters.lock().push(waiter.clone());
            this.waiter = Some(waiter);
            Poll::Pending
        }
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        value: Mutex::new(None),
        senders: AtomicUsize::new(1),
        waiters: Mutex::new(Vec::new()),
    });

    let sender = Sender {
        inner: Arc::downgrade(&inner),
    };

    let receiver = Receiver {
        waiter: None,
        inner,
    };

    (sender, receiver)
}

#[cfg(test)]
mod test {

    use futures::FutureExt;

    use super::*;
    #[tokio::test]
    async fn notify() {
        let (tx, mut rx) = channel();

        let mut rx2 = rx.clone();

        tx.send("foo".into());

        assert_eq!(
            rx.next_value().now_or_never(),
            Some(Some("foo".to_string()))
        );

        assert_eq!(rx2.next_value().now_or_never(), None);

        let task = tokio::spawn(async move {
            // let v = rx.next_value().await;
            // assert_eq!(v, None);
        });

        tokio::task::yield_now().await;

        tx.send("bar".into());
        drop(tx);

        assert_eq!(
            rx2.next_value().now_or_never(),
            Some(Some("bar".to_string()))
        );
        assert_eq!(rx2.next_value().now_or_never(), Some(None));

        tokio::task::yield_now().await;

        eprintln!("Joining task");
        assert!(task.now_or_never().is_some());
    }
}
