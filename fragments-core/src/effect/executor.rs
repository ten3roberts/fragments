use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
    task::{Context, Poll},
};

use flax::World;
use flume::{Receiver, Sender};
use futures::task::{waker_ref, ArcWake};
use slotmap::new_key_type;

use crate::App;

use super::Effect;

struct SharedTaskData {
    aborted: AtomicBool,
}

/// Represents a handle to a running task.
pub struct TaskHandle {
    shared: Weak<SharedTaskData>,
}

impl TaskHandle {
    pub fn abort_on_drop(self) -> AbortTaskHandle {
        AbortTaskHandle {
            shared: self.shared,
        }
    }
}

/// Variant of a task handle which aborts the task when dropped
pub struct AbortTaskHandle {
    shared: Weak<SharedTaskData>,
}

impl AbortTaskHandle {
    fn abort(&self) {
        if let Some(shared) = self.shared.upgrade() {
            shared.aborted.store(true, Ordering::SeqCst)
        }
    }
}

impl Drop for AbortTaskHandle {
    fn drop(&mut self) {
        self.abort()
    }
}

new_key_type! {
    struct TaskKey;
}

struct TaskWaker {
    key: TaskKey,
    pending: Sender<TaskKey>,
    sent: AtomicBool,
}

impl ArcWake for TaskWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        if arc_self
            .sent
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            arc_self.pending.send(arc_self.key).ok();
        }
    }
}

/// Represents a unit of effect execution which runs using `C`
pub(crate) struct Task {
    effect: Pin<Box<dyn Effect<App>>>,

    shared: Arc<SharedTaskData>,
}

impl Task {
    pub(crate) fn new(effect: Pin<Box<dyn Effect<App>>>) -> (Task, TaskHandle) {
        let shared = Arc::new(SharedTaskData {
            aborted: AtomicBool::new(false),
        });

        let handle = TaskHandle {
            shared: Arc::downgrade(&shared),
        };

        let task = Self { effect, shared };

        (task, handle)
    }

    fn update(&mut self, waker: &Arc<TaskWaker>, app: &mut App) -> Poll<()> {
        if self.shared.aborted.load(Ordering::Relaxed) {
            return Poll::Ready(());
        }

        let waker = waker_ref(waker);
        let mut cx = Context::from_waker(&waker);

        let effect = self.effect.as_mut();

        if effect.poll_effect(app, &mut cx).is_ready() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// Allows executing the app.
///
/// This allow executing the app, whereas the app only allows modification.
///
/// This is to avoid recursive updating, and enforcing an *ownership* of who can safely update the
/// app, and who can only act upon it.
pub struct AppExecutor {
    app: App,
    tasks: slotmap::SlotMap<TaskKey, (Task, Arc<TaskWaker>)>,
    /// Tasks which are ready to be polled
    pending_rx: Receiver<TaskKey>,
    pending_tx: Sender<TaskKey>,
    new_tasks_rx: Receiver<Task>,
}

impl std::ops::DerefMut for AppExecutor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.app
    }
}

impl std::ops::Deref for AppExecutor {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

impl AppExecutor {
    pub(crate) fn new(world: World) -> Self {
        let (pending_tx, pending_rx) = flume::unbounded();
        let (new_tasks_tx, new_tasks_rx) = flume::unbounded();

        let spawner = TaskSpawner { tx: new_tasks_tx };

        let app = App { world, spawner };

        Self {
            app,
            tasks: Default::default(),
            pending_rx,
            pending_tx,
            new_tasks_rx,
        }
    }

    /// Updates the app by executing all pending effects
    pub fn update(&mut self) {
        for new_task in self.new_tasks_rx.drain() {
            let key = self.tasks.insert_with_key(|key| {
                (
                    new_task,
                    Arc::new(TaskWaker {
                        key,
                        pending: self.pending_tx.clone(),
                        sent: AtomicBool::new(false),
                    }),
                )
            });

            self.pending_tx.send(key).ok();
        }

        for key in self.pending_rx.drain() {
            let Some((task, waker)) = self.tasks.get_mut(key) else { continue; };

            waker.sent.store(false, Ordering::SeqCst);

            if task.update(waker, &mut self.app).is_ready() {
                self.tasks.remove(key).unwrap();
            }
        }
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }
}

/// Allows spawning tasks
#[derive(Debug, Clone)]
pub struct TaskSpawner {
    tx: Sender<Task>,
}

impl TaskSpawner {
    /// Spawns a new task.
    pub fn spawn<E>(&self, effect: E) -> TaskHandle
    where
        E: 'static + Effect<App>,
    {
        let (task, handle) = Task::new(Box::pin(effect));

        self.tx.send(task).expect("Executor is not running");

        handle
    }
}
