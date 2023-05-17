use std::{
    cmp::Reverse,
    collections::{binary_heap::PeekMut, BinaryHeap},
    eprintln,
    marker::PhantomPinned,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
    thread::{self, Thread},
    time::Instant,
};

use futures::{
    task::{noop_waker, ArcWake},
    Future,
};
use parking_lot::Mutex;
use pin_project::{pin_project, pinned_drop};
use slotmap::{new_key_type, SlotMap};

struct TimerEntry {
    waker: Mutex<Waker>,
    finished: AtomicBool,
    _pinned: PhantomPinned,
}

new_key_type! {
    pub struct TimerKey;
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
struct Entry {
    deadline: Reverse<Instant>,
    key: TimerKey,
}

struct ThreadWaker {
    thread_id: Thread,
}

impl ArcWake for ThreadWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.thread_id.unpark()
    }
}

struct SharedTimer(*const TimerEntry);

unsafe impl Send for SharedTimer {}
unsafe impl Sync for SharedTimer {}

struct Inner {
    /// Invoked when there is a new timer
    waker: Waker,
    timers: SlotMap<TimerKey, SharedTimer>,
    heap: BinaryHeap<Entry>,
}

impl Inner {
    pub fn register(&mut self, deadline: Instant, timer: *const TimerEntry) -> TimerKey {
        let key = self.timers.insert(SharedTimer(timer));
        self.heap.push(Entry {
            deadline: Reverse(deadline),
            key,
        });

        eprintln!("Waking timers");
        self.waker.wake_by_ref();
        key
    }

    fn remove(&mut self, key: TimerKey) {
        eprintln!("Removing timer {key:?}");
        self.timers.remove(key);
    }
}

pub struct Timers {
    shared: Arc<Mutex<Inner>>,
}

impl Timers {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Mutex::new(Inner {
                timers: SlotMap::with_key(),
                heap: BinaryHeap::new(),
                waker: noop_waker(),
            })),
        }
    }

    /// Advances the timers, returning the next deadline
    pub fn tick(&mut self, time: Instant) -> Option<Instant> {
        let mut shared = self.shared.lock();
        let shared = &mut *shared;

        while let Some(entry) = shared.heap.peek_mut() {
            // All deadlines before now have been handled
            if entry.deadline.0 > time {
                eprintln!("Next deadline in {:?}", entry.deadline.0 - time);
                return Some(entry.deadline.0);
            }

            let entry = PeekMut::pop(entry);
            let key = entry.key;
            if let Some(entry) = shared.timers.get(key) {
                eprintln!("Waking timer: {key:?}");
                // Fire and wake the timer
                // # Safety
                // Sleep removes the timer when dropped
                // Drop is guaranteed due to Sleep being pinned when registered
                let entry = unsafe { &*(entry.0) };

                entry.finished.store(true, Ordering::Release);
                entry.waker.lock().wake_by_ref();
            } else {
                eprintln!("Timer was dead")
            }
        }

        None
    }

    pub fn run_blocking(mut self) {
        let waker = Arc::new(ThreadWaker {
            thread_id: thread::current(),
        });

        self.shared.lock().waker = futures::task::waker(waker);

        loop {
            let now = Instant::now();
            let next = self.tick(now);

            if let Some(next) = next {
                thread::park_timeout(next - now)
            } else {
                thread::park()
            }
        }
    }
}

#[pin_project(PinnedDrop)]
struct Sleep {
    shared: Arc<Mutex<Inner>>,
    timer: TimerEntry,
    deadline: Instant,
    key: Option<TimerKey>,
}

impl Sleep {
    fn new(shared: Arc<Mutex<Inner>>, deadline: Instant) -> Self {
        Self {
            shared,
            timer: TimerEntry {
                waker: Mutex::new(noop_waker()),
                finished: AtomicBool::new(false),
                _pinned: PhantomPinned,
            },
            deadline,
            key: None,
        }
    }

    fn register(self: Pin<&mut Self>) {
        let p = self.project();
        assert!(p.key.is_none());
        let key = p.shared.lock().register(*p.deadline, p.timer);
        *p.key = Some(key);
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self
            .timer
            .finished
            .compare_exchange(true, false, Ordering::Release, Ordering::Relaxed)
            .is_ok()
        {
            Poll::Ready(())
        } else if self.key.is_none() {
            *self.timer.waker.lock() = cx.waker().clone();
            self.register();

            Poll::Pending
        } else {
            *self.timer.waker.lock() = cx.waker().clone();
            Poll::Pending
        }
    }
}

#[pinned_drop]
impl PinnedDrop for Sleep {
    fn drop(self: Pin<&mut Self>) {
        if let Some(key) = self.key {
            let mut shared = self.shared.lock();
            shared.remove(key);
        }
    }
}

impl Default for Timers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use std::{eprintln, time::Duration};

    use futures::FutureExt;

    use super::*;

    fn assert_dur(found: Duration, expected: Duration) {
        assert!(
            (found.as_millis().abs_diff(expected.as_millis())) < 10,
            "Expected {:?} to be close to {:?}",
            found,
            expected,
        )
    }

    #[test]
    fn sleep() {
        let timers = Timers::new();

        let shared = timers.shared.clone();

        thread::spawn(move || timers.run_blocking());

        let now = Instant::now();
        futures::executor::block_on(async move {
            Sleep::new(shared.clone(), Instant::now() + Duration::from_millis(500)).await;

            eprintln!("Timer 1 finished");

            let now = Instant::now();
            Sleep::new(shared.clone(), Instant::now() + Duration::from_millis(1000)).await;

            Sleep::new(shared.clone(), now - Duration::from_millis(100)).await;

            eprintln!("Expired timer finished")
        });

        assert_dur(now.elapsed(), Duration::from_millis(500 + 1000));

        eprintln!("Done");
    }

    #[test]
    fn sleep_join() {
        let timers = Timers::new();

        let shared = timers.shared.clone();

        thread::spawn(move || timers.run_blocking());

        let now = Instant::now();
        futures::executor::block_on(async move {
            let sleep_1 = Sleep::new(shared.clone(), Instant::now() + Duration::from_millis(500));

            eprintln!("Timer 1 finished");

            let now = Instant::now();
            let sleep_2 = Sleep::new(shared.clone(), Instant::now() + Duration::from_millis(1000));

            let sleep_3 = Sleep::new(shared.clone(), now - Duration::from_millis(100));

            futures::join!(sleep_1, sleep_2, sleep_3);

            eprintln!("Expired timer finished")
        });

        assert_dur(now.elapsed(), Duration::from_millis(1000));
        eprintln!("Done");
    }

    #[test]
    fn sleep_race() {
        let timers = Timers::new();

        let shared = timers.shared.clone();

        thread::spawn(move || timers.run_blocking());

        let now = Instant::now();
        futures::executor::block_on(async move {
            {
                let mut sleep_1 =
                    Sleep::new(shared.clone(), Instant::now() + Duration::from_millis(500)).fuse();

                eprintln!("Timer 1 finished");

                let mut sleep_2 =
                    Sleep::new(shared.clone(), Instant::now() + Duration::from_millis(1000)).fuse();

                futures::select!(_ = sleep_1 => {}, _ = sleep_2 => {});
            }

            Sleep::new(shared.clone(), Instant::now() + Duration::from_millis(1500)).await;
        });

        assert_dur(now.elapsed(), Duration::from_millis(2000));

        eprintln!("Done");
    }
}
