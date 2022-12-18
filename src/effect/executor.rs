use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Weak,
    },
    task::Context,
};

use flume::{Receiver, Sender};
use futures::task::{waker_ref, ArcWake};
use parking_lot::Mutex;

use crate::App;

use super::Effect;

const STATE_PENDING: u8 = 1;
const STATE_READY: u8 = 2;
const STATE_ABORTED: u8 = 3;
const STATE_FINISHED: u8 = 4;

/// Represents a handle to a running task.
pub struct TaskHandle<T> {
    inner: Weak<Task<T>>,
}

impl<T> TaskHandle<T> {
    pub fn abort_on_drop(self) -> AbortTaskHandle<T> {
        AbortTaskHandle { inner: self.inner }
    }
}

/// Variant of a task handle which aborts the task when dropped
pub struct AbortTaskHandle<T> {
    inner: Weak<Task<T>>,
}

impl<T> AbortTaskHandle<T> {
    fn abort(&self) {
        if let Some(inner) = self.inner.upgrade() {
            inner.state.store(STATE_ABORTED, Ordering::SeqCst)
        }
    }
}

impl<T> Drop for AbortTaskHandle<T> {
    fn drop(&mut self) {
        self.abort()
    }
}

/// Represents a unit of effect execution which runs using `C`
pub(crate) struct Task<T> {
    effect: Mutex<Pin<Box<dyn Effect<App> + Send>>>,
    queue: Sender<Arc<Self>>,
    state: AtomicU8,
}

impl<T> ArcWake for Task<T> {
    fn wake_by_ref(arc_self: &Arc<Self>) {
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
            arc_self.queue.send(arc_self.clone()).ok();
        }
    }
}

impl<T> Task<T> {
    pub(crate) fn new(
        effect: Pin<Box<dyn Effect<App> + Send>>,
        queue: Sender<Arc<Self>>,
    ) -> (Arc<Self>, TaskHandle<T>) {
        let this = Arc::new(Self {
            effect: Mutex::new(effect),
            queue,
            state: AtomicU8::new(STATE_READY),
        });

        let handle = TaskHandle {
            inner: Arc::downgrade(&this),
        };

        (this, handle)
    }

    pub fn run(self: &Arc<Self>, app: &mut App) {
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
            let waker = waker_ref(self);
            let mut cx = Context::from_waker(&waker);

            let mut effect = self.effect.lock();
            let effect = effect.as_mut();

            if effect.poll_effect(app, &mut cx).is_ready() {
                self.state.store(STATE_FINISHED, Ordering::SeqCst);
            }
        }
    }
}

/// Executes application level effect tasks.
///
/// All scope tasks are lifted to an app level effect
pub struct AppExecutor {
    tx: EffectSender,
    rx: EffectReceiver,
}

impl AppExecutor {
    /// Runs all pending effects using the provided app
    pub fn run(&self, app: &mut App) {
        for task in self.rx.drain() {
            task.run(app)
        }
    }
}

pub(crate) type EffectSender = Sender<Arc<Task<()>>>;
pub(crate) type EffectReceiver = Receiver<Arc<Task<()>>>;
