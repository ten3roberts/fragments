use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use atomic_refcell::AtomicRefCell;
use flume::{Receiver, Sender};
use futures::task::{waker_ref, ArcWake};
use pin_project::pin_project;

use crate::app::App;

use super::Signal;

/// A `task` which runs on the world
pub(crate) trait Effect: 'static + Send + Sync {
    fn poll_effect(self: Arc<Self>, app: &mut App);
    fn abort(&self);
}

const STATE_PENDING: u8 = 1;
const STATE_READY: u8 = 2;
const STATE_ABORTED: u8 = 3;

/// An effect which queues itself for each item in the signal
#[pin_project]
pub(crate) struct SignalEffect<S, F> {
    queue: Sender<Arc<dyn Effect>>,
    state: AtomicU8,
    handler: AtomicRefCell<F>,
    #[pin]
    signal: AtomicRefCell<S>,
}

impl<S, F> SignalEffect<S, F> {
    pub fn new(queue: Sender<Arc<dyn Effect>>, signal: S, handler: F) -> Self {
        Self {
            queue,
            state: AtomicU8::new(STATE_READY),
            handler: AtomicRefCell::new(handler),
            signal: AtomicRefCell::new(signal),
        }
    }
}

impl<S, F> ArcWake for SignalEffect<S, F>
where
    S: 'static + Send + Sync + for<'x> Signal<'x>,
    F: 'static + Send + Sync + for<'x> FnMut(&mut App, <S as Signal<'x>>::Item),
{
    fn wake_by_ref(arc_self: &Arc<Self>) {
        eprintln!("Waking effect");
        if arc_self
            .state
            .compare_exchange(
                STATE_PENDING,
                STATE_READY,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            eprintln!("Enqueueing task");
            arc_self
                .queue
                .send(arc_self.clone() as Arc<dyn Effect>)
                .ok();
        } else {
            eprintln!("Already enqueued or aborted")
        }
    }
}

impl<S, F> Effect for SignalEffect<S, F>
where
    S: 'static + Send + Sync + for<'x> Signal<'x>,
    F: 'static + Send + Sync + for<'x> FnMut(&mut App, <S as Signal<'x>>::Item),
{
    fn poll_effect(self: Arc<Self>, app: &mut App) {
        if self
            .state
            .compare_exchange(
                STATE_READY,
                STATE_PENDING,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            eprintln!("Effect ready");
            let waker = waker_ref(&self);
            let mut cx = Context::from_waker(&waker);

            {
                let signal = self.signal.borrow_mut();
                // # Safety
                // The signal is never moved or replaced
                let mut signal = unsafe { Pin::new_unchecked(signal) };
                {
                    while let Poll::Ready(Some(v)) = signal.as_mut().poll_changed(&mut cx) {
                        (self.handler.borrow_mut())(app, v);
                    }
                }
            }
        }
    }

    fn abort(&self) {
        eprintln!("Aborting effect");
        self.state.store(STATE_ABORTED, Ordering::SeqCst);
    }
}

pub(crate) type EffectSender = Sender<Arc<dyn Effect>>;
pub(crate) type EffectReceiver = Receiver<Arc<dyn Effect>>;
