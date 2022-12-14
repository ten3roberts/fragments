use std::sync::Arc;

use flax::{child_of, Component, ComponentValue, Entity, EntityBuilder, EntityRef, World};

use crate::{
    app::{
        effect::{Effect, SignalEffect},
        App,
    },
    signal::Signal,
};

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
    id: Entity,
    app: &'a mut App,
}

impl<'a> Scope<'a> {
    /// Creates a new scope
    pub(crate) fn spawn(app: &'a mut App, parent: Option<Entity>) -> Self {
        let mut data = Entity::builder();
        if let Some(parent) = parent {
            data.tag(child_of(parent));
        }

        let id = data.spawn(app.world_mut());
        Self::reconstruct(app, id).unwrap()
    }

    pub fn create_effect<S, T, F>(&mut self, signal: S, mut effect: F)
    where
        S: 'static + Send + Sync + for<'x> Signal<'x, Item = T>,
        F: FnMut(Scope<'_>, T) + 'static + Send + Sync,
    {
        let id = self.id;
        let effects = self.app.effects_tx().clone();
        let signal = Arc::new(SignalEffect::new(
            effects,
            signal,
            Box::new(move |app: &mut App, item: S::Item| {
                if let Some(s) = Scope::reconstruct(app, id) {
                    effect(s, item);
                }
            }),
        )) as Arc<dyn Effect>;

        // Abort the effect when despawning the entity
        // self.app
        //     .world_mut()
        //     .entry(self.id, abort_on_drop())
        //     .unwrap()
        //     .or_default()
        //     .push(Arc::downgrade(&signal));

        self.app.effects_tx().send(signal).ok();
    }

    /// Reconstruct the scope for an entity
    fn reconstruct(app: &'a mut App, id: Entity) -> Option<Self> {
        if !app.world().is_alive(id) {
            return None;
        }

        Some(Self { id, app })
    }

    /// Set a component for the widget
    pub fn set<T: ComponentValue>(&mut self, component: Component<T>, value: T) -> &mut Self {
        self.app.world_mut().set(self.id, component, value).unwrap();
        self
    }

    pub fn entity_ref(&mut self) -> EntityRef {
        self.app.world_mut().entity(self.id).unwrap()
    }

    pub fn app_mut(&mut self) -> &mut App {
        self.app
    }

    pub fn app(&self) -> &&'a mut App {
        &self.app
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
