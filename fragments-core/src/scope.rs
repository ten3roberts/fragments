use std::task::{Context, Poll};

use flax::{child_of, Component, ComponentValue, Entity, EntityRef, EntityRefMut, World};
use futures::{Future, Stream};
use pin_project::pin_project;

use crate::{
    components::tasks,
    effect::{Effect, FutureEffect, SignalEffect, StreamEffect, TaskSpawner},
    events::{EventHandler, EventState},
    signal::Signal,
    App, Widget,
};

/// Represents the scope of a widget.
pub struct Scope<'a> {
    entity: EntityRefMut<'a>,
    spawner: &'a TaskSpawner,
}

impl<'a> Scope<'a> {
    /// Creates a new scope
    pub(crate) fn spawn(
        world: &'a mut World,
        spawner: &'a TaskSpawner,
        parent: Option<Entity>,
    ) -> Self {
        let mut entity = world.spawn_ref();
        if let Some(parent) = parent {
            entity.set(child_of(parent), ());
        }

        Self { entity, spawner }
    }

    pub fn use_signal<S, F, T>(&mut self, signal: S, func: F)
    where
        S: 'static + for<'x> Signal<'x, Item = T>,
        F: 'static + FnMut(&mut Scope<'_>, T),
    {
        self.use_effect(SignalEffect::new(signal, func))
    }

    pub fn use_future<Fut, F>(&mut self, fut: Fut, func: F)
    where
        Fut: 'static + Future,
        F: 'static + FnMut(&mut Scope<'_>, Fut::Output),
    {
        self.use_effect(FutureEffect::new(fut, func))
    }

    pub fn use_stream<S, F>(&mut self, fut: S, func: F)
    where
        S: 'static + Stream,
        F: 'static + FnMut(&mut Scope<'_>, S::Item),
    {
        self.use_effect(StreamEffect::new(fut, func))
    }

    /// Spawns the effect inside the given scope.
    ///
    /// Returns a handle which will control the effect
    pub fn use_effect<E>(&mut self, effect: E)
    where
        E: 'static + for<'x> Effect<Scope<'x>>,
    {
        // lift App => Scope
        let effect = MapContextScope {
            id: self.entity.id(),
            effect,
        };

        let handle = self.spawner.spawn(effect);

        // Abort the effect when despawning the entity
        self.entity.entry_ref(tasks()).or_default().push(handle);
    }

    /// Listener for a global event
    ///
    /// The event handlers run without mutable access to the world, and can as such not attach new
    /// children.
    pub fn on_global_event<E>(
        &mut self,
        event_kind: Component<EventHandler<E>>,
        event_handler: impl 'static + Send + Sync + FnMut(EntityRef, &E),
    ) -> &mut Self
    where
        E: 'static,
    {
        self.set(event_kind, Box::new(event_handler));
        self
    }

    /// Reconstruct the scope for an entity
    fn reconstruct(world: &'a mut World, spawner: &'a TaskSpawner, id: Entity) -> Option<Self> {
        let entity = world.entity_mut(id).ok()?;

        Some(Self { entity, spawner })
    }

    /// Set a component for the widget
    pub fn set<T: ComponentValue>(&mut self, component: Component<T>, value: T) -> &mut Self {
        self.entity.set(component, value);
        self
    }

    pub fn attach_child<W: Widget>(&mut self, widget: W) -> Entity {
        let id = self.entity.id();
        let mut child_scope = Scope::spawn(self.entity.world_mut(), self.spawner, Some(id));
        let id = child_scope.id();

        widget.render(&mut child_scope);
        id
    }

    /// Returns the entity id
    pub fn id(&self) -> Entity {
        self.entity.id()
    }

    /// Returns the underlying entity for the scope
    pub fn entity(&self) -> &EntityRefMut<'a> {
        &self.entity
    }

    /// Returns the underlying entity for the scope
    pub fn entity_mut(&mut self) -> &mut EntityRefMut<'a> {
        &mut self.entity
    }
}

/// Lifts a scope local effect to the world.
#[pin_project]
struct MapContextScope<E> {
    #[pin]
    effect: E,
    id: Entity,
}

impl<E> Effect<App> for MapContextScope<E>
where
    E: for<'x> Effect<Scope<'x>>,
{
    fn poll_effect(
        self: std::pin::Pin<&mut Self>,
        app: &mut App,
        async_cx: &mut Context<'_>,
    ) -> Poll<()> {
        let world = &mut app.world;
        let spawner = &app.spawner;

        let scope = Scope::reconstruct(world, spawner, self.id);

        if let Some(mut scope) = scope {
            self.project().effect.poll_effect(&mut scope, async_cx)
        } else {
            Poll::Ready(())
        }
    }
}
