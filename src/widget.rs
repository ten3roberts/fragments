use std::sync::{Arc, MutexGuard};

use flax::{child_of, Component, ComponentValue, Entity, EntityBuilder, EntityRefMut, World};
use flume::Sender;
use futures::{Future, StreamExt};
use futures_signals::signal::{Signal, SignalExt};
use parking_lot::Mutex;
use slotmap::SlotMap;
use tokio::runtime::Handle;

use crate::{app::App, AppEvent, Effect, EffectKey};

pub trait Widget {
    fn render(self, scope: Scope);
}

impl<F> Widget for F
where
    F: FnMut(Scope<'_>),
{
    fn render(mut self, scope: Scope) {
        (self)(scope)
    }
}

pub struct Scope<'a> {
    entity: EntityRefMut<'a>,
    app: &'a App,
}

impl<'a> Scope<'a> {
    /// Creates a new scope
    pub(crate) fn new(app: &'a App, world: &'a mut World, parent: Option<Entity>) -> Self {
        let mut data = Entity::builder();
        if let Some(parent) = parent {
            data.tag(child_of(parent));
        }

        let id = data.spawn(world);
        Self::reconstruct(app, world, id).unwrap()
    }

    pub fn create_effect<S>(
        &mut self,
        signal: S,
        mut effect: impl FnMut(Scope<'_>, S::Item) + 'static + Send,
    ) where
        S: 'static + Send + Signal,
        S::Item: 'static + Send,
    {
        let id = self.entity.id();
        self.app.create_effect(signal, |app, world, item| {
            let s = match Self::reconstruct(app, world, id) {
                Some(v) => v,
                None => return false,
            };

            effect(s, item);
            true
        })
    }

    /// Reconstruct the scope for an entity
    fn reconstruct(app: &'a App, world: &'a mut World, id: Entity) -> Option<Self> {
        let rt = app.runtime();

        let entity = world.entity_mut(id).ok()?;
        let effects = &mut app.effects;
        let tx = &mut app.events_tx;

        Some(Self { entity, app })
    }

    /// Set a component for the widget
    pub fn set<T: ComponentValue>(&mut self, component: Component<T>, value: T) -> &mut Self {
        self.entity.set(component, value).unwrap();
        self
    }

    pub fn spawn_task<F: 'static + Future<Output = ()> + Send>(&self, future: F) {
        self.app.runtime().spawn(future);
    }
}

pub struct Fragment {
    data: EntityBuilder,
}

impl Fragment {
    pub fn spawn(mut self, world: &mut World) -> Entity {
        self.data.spawn(world)
    }
}
