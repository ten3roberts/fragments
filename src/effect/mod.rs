mod future;
mod signal;
mod stream;

use atomic_refcell::AtomicRefCell;
pub use future::*;
pub(crate) use signal::*;
pub use stream::*;

use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering::*},
        Arc,
    },
    task::Context,
};

use flume::{Receiver, Sender};
use futures::task::{waker_ref, ArcWake};

use crate::app::App;

/// A `task` which runs on the world
pub(crate) trait Effect: 'static + Send + Sync {
    fn poll_effect(self: Pin<&mut Self>, app: &mut App, cx: &mut Context<'_>);
    fn abort(&self);
}

const STATE_PENDING: u8 = 1;
const STATE_READY: u8 = 2;
const STATE_ABORTED: u8 = 3;

/// An effect which queues itself for each item in the signal
pub(crate) struct EffectExecutor {
    effect: AtomicRefCell<Pin<Box<dyn Effect>>>,
    queue: Sender<Arc<EffectExecutor>>,
    ready: AtomicBool,
}

impl ArcWake for EffectExecutor {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        if arc_self
            .ready
            .compare_exchange(false, true, Acquire, Relaxed)
            .is_ok()
        {
            eprintln!("Enqueueing task");
            arc_self.queue.send(arc_self.clone()).ok();
        } else {
            eprintln!("Already enqueued or aborted")
        }
    }
}

impl EffectExecutor {
    pub(crate) fn new(effect: Pin<Box<dyn Effect>>, queue: Sender<Arc<EffectExecutor>>) -> Self {
        Self {
            effect: AtomicRefCell::new(effect),
            queue,
            ready: AtomicBool::new(true),
        }
    }

    pub fn run(self: &Arc<Self>, app: &mut App) {
        if self
            .ready
            .compare_exchange(true, false, Acquire, Relaxed)
            .is_ok()
        {
            let waker = waker_ref(self);
            let mut cx = Context::from_waker(&waker);

            let mut effect = self.effect.borrow_mut();
            let effect = effect.as_mut();
            effect.poll_effect(app, &mut cx)
        }
    }
}

// impl<S, F> Effect for SignalEffect<S, F>
// where
//     S: 'static + Send + Sync + for<'x> Signal<'x>,
//     F: 'static + Send + Sync + for<'x> FnMut(&mut App, <S as Signal<'x>>::Item),
// {
//     fn poll_effect(self: Arc<Self>, app: &mut App) {
//         if self
//             .state
//             .compare_exchange(
//                 STATE_READY,
//                 STATE_PENDING,
//                 Ordering::Acquire,
//                 Ordering::Relaxed,
//             )
//             .is_ok()
//         {
//             eprintln!("Effect ready");
//             let _self = self.clone();

//             let waker = waker_ref(&self);
//             let mut cx = Context::from_waker(&waker);

//             {
//                 let signal = self.signal.borrow_mut();
//                 // # Safety
//                 // The signal is never moved or replaced
//                 let mut signal = unsafe { Pin::new_unchecked(signal) };
//                 {
//                     while let Poll::Ready(Some(v)) = signal.as_mut().poll_changed(&mut cx) {
//                         (self.handler.borrow_mut())(app, v);
//                     }
//                 }
//             }
//         }
//     }

//     fn abort(&self) {
//         eprintln!("Aborting effect");
//         self.state.store(STATE_ABORTED, Ordering::SeqCst);
//     }
// }

pub(crate) type EffectSender = Sender<Arc<EffectExecutor>>;
pub(crate) type EffectReceiver = Receiver<Arc<EffectExecutor>>;

