use crate::{
    effect::{Task, TaskHandle},
    error::Error,
    Scope, Widget,
};
use flax::World;

use futures::StreamExt;
use slotmap::new_key_type;
use tokio::runtime::Runtime;

use crate::effect::{Effect, EffectReceiver, EffectSender};

new_key_type! { pub struct EffectKey; }

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

#[derive(Debug)]
pub struct App {
    world: World,
    effects_tx: EffectSender,
    effects_rx: EffectReceiver,
    runtime: Runtime,
}

impl App {
    pub fn new() -> Self {
        let (effects_tx, effects_rx) = flume::unbounded();

        Self {
            world: Default::default(),
            runtime: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
            effects_tx,
            effects_rx,
        }
    }

    pub fn update(&mut self) {
        while let Ok(effect) = self.effects_rx.try_recv() {
            effect.run(self);
        }
    }

    /// Enters the render loop
    pub fn run(mut self, root: impl Widget) -> Result<(), Error> {
        let rt = self.runtime.handle().clone();
        rt.block_on(async move {
            let scope = Scope::spawn(&mut self, None);
            root.render(scope);

            let mut pending_effects = self.effects_rx.clone().into_stream();
            eprintln!("Waiting for pending effects");
            while let Some(effect) = pending_effects.next().await {
                effect.run(&mut self);
            }
        });
        Ok(())
    }

    /// Spawns the effect into the app.
    ///
    /// Returns a handle which will control the effect
    pub(crate) fn spawn_effect<E: 'static + Effect<Output = ()> + Send>(
        &self,
        effect: E,
    ) -> TaskHandle<()> {
        let (task, handle) = Task::new(Box::pin(effect), self.effects_tx.clone());

        self.effects_tx.send(task).ok();
        handle
    }

    pub(crate) fn effects_tx(&self) -> &EffectSender {
        &self.effects_tx
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
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
