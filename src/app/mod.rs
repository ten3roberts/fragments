use crate::{error::Error, Scope, Widget};
use flax::World;

use futures::StreamExt;
use slotmap::new_key_type;
use std::sync::Arc;
use tokio::runtime::Runtime;

use self::effect::{Effect, EffectReceiver, EffectSender};
pub mod effect;

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
            effect.poll_effect(self);
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
                effect.poll_effect(&mut self);
            }
        });
        Ok(())
    }

    pub(crate) fn run_effect<E: Effect>(&self, effect: Arc<E>) {
        self.effects_tx.send(effect).ok();
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
