use crate::{Scope, Widget};
use flax::World;

use futures::future::BoxFuture;
use slotmap::new_key_type;

use crate::effect::{EffectReceiver, EffectSender};

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

pub struct HeadlessBackend;

impl Backend for HeadlessBackend {
    type Output = BoxFuture<'static, ()>;

    fn run(self, mut app: App) -> BoxFuture<'static, ()> {
        Box::pin(async move {
            loop {
                app.update_async().await;
            }
        })
    }
}

pub trait Backend {
    type Output;
    fn run(self, app: App) -> Self::Output;
}

pub struct AppBuilder<T> {
    backend: T,
}

impl<T> AppBuilder<T>
where
    T: Backend,
{
    /// Runs the app
    pub fn run(self) -> T::Output {
        let (tx, rx) = flume::unbounded();

        let mut app = App {
            world: World::new(),
            effects_tx: tx,
            effects_rx: rx,
        };

        self.backend.run(app)
    }
}

#[derive(Debug)]
pub struct App {
    pub(crate) world: World,
    pub(crate) effects_tx: EffectSender,
    pub(crate) effects_rx: EffectReceiver,
}

impl App {
    pub fn builder<T>(backend: T) -> AppBuilder<T> {
        AppBuilder { backend }
    }

    pub fn update(&mut self) {
        for effect in self.effects_rx.clone().drain() {
            effect.update(self);
        }
    }

    pub async fn update_async(&mut self) {
        while let Ok(effect) = self.effects_rx.recv_async().await {
            effect.update(self);
        }
    }

    pub fn attach_root(&mut self, root:impl Widget){
        let scope = Scope::spawn(&mut self.world, &self.effects_tx, None);
        root.render(scope);

      }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}

pub enum AppEvent {
    Exit,
}
