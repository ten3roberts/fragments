use std::{
    mem,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
    task::{Context, Poll, Waker},
};

use futures::task::{waker_ref, ArcWake};
use parking_lot::Mutex;
use slotmap::new_key_type;

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
    shared: Arc<Shared>,
    sent: AtomicBool,
}

impl ArcWake for TaskWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        if arc_self
            .sent
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            arc_self.shared.push_ready(arc_self.key);
        }
    }
}

/// Represents a unit of effect execution which runs using `C`
pub(crate) struct Task<T> {
    effect: Pin<Box<dyn Effect<T>>>,

    shared: Arc<SharedTaskData>,
}

impl<T> Task<T> {
    pub(crate) fn new(effect: Pin<Box<dyn Effect<T>>>) -> (Task<T>, TaskHandle) {
        let shared = Arc::new(SharedTaskData {
            aborted: AtomicBool::new(false),
        });

        let handle = TaskHandle {
            shared: Arc::downgrade(&shared),
        };

        let task = Self { effect, shared };

        (task, handle)
    }

    fn update(&mut self, waker: &Arc<TaskWaker>, state: &mut T) -> Poll<()> {
        if self.shared.aborted.load(Ordering::Relaxed) {
            return Poll::Ready(());
        }

        let waker = waker_ref(waker);
        let mut cx = Context::from_waker(&waker);

        let effect = self.effect.as_mut();

        if effect.poll_effect(state, &mut cx).is_ready() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

struct Shared {
    /// Task which are ready to be polled again
    ready: Mutex<Vec<TaskKey>>,
    waker: Mutex<Option<Waker>>,
    has_updates: AtomicBool,
}

impl Shared {
    pub fn push_ready(&self, key: TaskKey) {
        self.ready.lock().push(key);
        self.wake();
    }

    fn wake(&self) {
        self.has_updates.store(true, Ordering::SeqCst);
        if let Some(waker) = &mut *self.waker.lock() {
            waker.wake_by_ref();
        }
    }
}

/// Executes `Tasks`
pub struct Executor<T> {
    /// Tasks are stored inline
    tasks: slotmap::SlotMap<TaskKey, (Task<T>, Arc<TaskWaker>)>,
    new_tasks: Arc<Mutex<Vec<Task<T>>>>,
    processing: Vec<TaskKey>,
    shared: Arc<Shared>,
}

impl<T> Executor<T> {
    pub fn new() -> Self {
        let shared = Arc::new(Shared {
            ready: Default::default(),
            waker: Default::default(),
            has_updates: AtomicBool::new(false),
        });

        Self {
            tasks: Default::default(),
            new_tasks: Default::default(),
            processing: Default::default(),
            shared,
        }
    }

    /// Poll until there are tasks ready to update
    pub fn poll_update(&mut self, cx: Context<'_>, state: &mut T) -> Poll<()> {
        if self
            .shared
            .has_updates
            .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            self.update(state);
            Poll::Ready(())
        } else {
            *self.shared.waker.lock() = Some(cx.waker().clone());
            Poll::Pending
        }
    }

    pub fn spawner(&self) -> TaskSpawner<T> {
        TaskSpawner {
            shared: Arc::downgrade(&self.shared),
            new_tasks: Arc::downgrade(&self.new_tasks.clone()),
        }
    }

    /// Updates the executor, polling ready tasks using the provided state
    pub fn update(&mut self, state: &mut T) {
        self.shared.has_updates.store(false, Ordering::SeqCst);
        mem::swap(&mut *self.shared.ready.lock(), &mut self.processing);

        // Drain all new tasks and put them into the slotmap
        for new_task in self.new_tasks.lock().drain(..) {
            let key = self.tasks.insert_with_key(|key| {
                (
                    new_task,
                    Arc::new(TaskWaker {
                        key,
                        shared: self.shared.clone(),
                        sent: AtomicBool::new(false),
                    }),
                )
            });

            self.processing.push(key);
        }

        for key in self.processing.drain(..) {
            let Some((task, waker)) = self.tasks.get_mut(key) else { continue; };

            // Reset the waker so that it is ready to use again
            waker.sent.store(false, Ordering::SeqCst);

            // Poll the task, removing the task if ready
            if task.update(waker, state).is_ready() {
                self.tasks.remove(key).unwrap();
            }
        }
    }
}

/// Allows spawning tasks
#[derive(Debug, Clone)]
pub struct TaskSpawner<T> {
    new_tasks: Weak<Mutex<Vec<Task<T>>>>,
    shared: Weak<Shared>,
}

impl<T> TaskSpawner<T> {
    /// Spawns a new task.
    pub fn spawn<E>(&self, effect: E) -> TaskHandle
    where
        E: 'static + Effect<T>,
    {
        let shared = self.shared.upgrade().expect("No executor running");
        let new_tasks = self.new_tasks.upgrade().expect("No executor running");

        let (task, handle) = Task::new(Box::pin(effect));

        new_tasks.lock().push(task);
        shared.wake();

        handle
    }
}
