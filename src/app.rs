use crate::{error::Error, Scope, Widget};
use dashmap::DashSet;
use flax::World;
use flume::{Receiver, Sender};
use futures::{Future, StreamExt};
use futures_signals::signal::{Signal, SignalExt};
use parking_lot::Mutex;
use slotmap::{new_key_type, SlotMap};
use std::sync::Arc;
use tokio::runtime::{Handle, Runtime};

new_key_type! { pub struct EffectKey; }

pub type Effect = Box<dyn FnMut(&mut App, &mut World)>;

// trait Effect {
//     /// Executes the effect on the world
//     fn run(&mut self, world: &mut World);
// }

// struct SignalEffect<S, F>
// where
//     S: Signal,
// {
//     // This value will be updated by the spawned future
//     value: Arc<Mutex<Option<S::Item>>>,
//     func: F,
// }

// impl<S, F> Effect for SignalEffect<S, F>
// where
//     S: Signal,
//     F: FnMut(&mut World, S::Item),
// {
//     fn run(&mut self, world: &mut World) {
//         (self.func)(world, self.value.lock().take().unwrap());
//     }
// }

pub struct App {
    world: World,
    runtime: Runtime,
}

impl App {
    pub fn new() -> Self {
        Self {
            world: Default::default(),
            runtime: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
        }
    }

    /// Enters the render loop
    pub fn run(mut self, root: impl Widget) -> Result<(), Error> {
        let app = Arc::new(Mutex::new(self));
        let mut world = World::new();
        let scope = Scope::new(&mut self, &mut world, None);
        root.render(scope);

        let rt = self.runtime.handle().clone();

        Ok(())
    }

    pub fn create_effect<S>(
        &mut self,
        signal: S,
        mut effect: impl FnMut(&mut World, S::Item) -> bool + 'static + Send,
    ) where
        S: 'static + Send + Signal,
        S::Item: 'static + Send,
    {
        let value: Arc<Mutex<Option<S::Item>>> = Arc::new(Mutex::new(None));

        let v = value.clone();

        let world = self.world.clone();
        self.runtime.spawn(async move {
            let items = signal.to_stream();
            tokio::pin!(items);
            loop {
                while let Some(item) = items.next().await {
                    let world = world.lock();
                    effect() * value.lock() = Some(item);
                    match tx.send(AppEvent::RunEffect(effect)) {
                        Ok(()) => {}
                        Err(_) => break,
                    };
                }
            }
        });
    }
    pub fn runtime(&self) -> &Handle {
        self.rt.handle()
    }

    pub fn events_tx(&self) -> &Sender<AppEvent> {
        &self.events_tx
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

pub enum AppEvent {
    Exit,
}
