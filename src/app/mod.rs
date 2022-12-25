use std::time::Duration;

use crate::{
    effect::{AppExecutor, TaskSpawner},
    Scope, Widget,
};
use flax::{Entity, World};

use futures::future::LocalBoxFuture;
use slotmap::new_key_type;
use tokio::time::interval;

new_key_type! { pub struct EffectKey; }

pub struct HeadlessBackend;

impl Backend for HeadlessBackend {
    type Output = LocalBoxFuture<'static, ()>;

    fn run<W: Widget>(self, mut app: AppExecutor, root: W) -> Self::Output {
        app.attach_root(root);
        Box::pin(async move {
            let mut interval = interval(Duration::from_millis(100));
            loop {
                interval.tick().await;
                app.update();
            }
        })
    }
}

pub trait Backend {
    type Output;
    /// Enter the main backend loop using the app and provided root widget.
    ///
    /// After initialization, the root must be attached.
    fn run<W: Widget>(self, app: AppExecutor, root: W) -> Self::Output;
}

pub struct AppBuilder<T> {
    backend: T,
}

impl<T> AppBuilder<T>
where
    T: Backend,
{
    /// Runs the app
    pub fn run(self, root: impl Widget) -> T::Output {
        let executor = AppExecutor::new(World::new());

        self.backend.run(executor, root)
    }
}

#[derive(Debug)]
pub struct App {
    pub(crate) world: World,
    pub(crate) spawner: TaskSpawner,
}

impl App {
    pub fn builder<T>(backend: T) -> AppBuilder<T> {
        AppBuilder { backend }
    }

    /// Attaches a new root widget
    pub fn attach_root(&mut self, root: impl Widget) -> Entity {
        let mut scope = Scope::spawn(&mut self.world, &self.spawner, None);
        let id = scope.entity().id();
        root.render(&mut scope);
        id
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
