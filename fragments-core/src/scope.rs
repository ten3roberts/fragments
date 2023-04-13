use std::task::{Context, Poll};

use atomic_refcell::AtomicRef;
use flax::{child_of, Component, ComponentValue, Entity, EntityRef, EntityRefMut, World};
use futures::{Future, Stream};
use pin_project::pin_project;

use crate::{
    components::tasks,
    context::ContextKey,
    effect::{Effect, FutureEffect, SignalEffect, StreamEffect, TaskSpawner},
    events::EventHandler,
    frame::Frame,
    signal::Signal,
    Widget,
};

/// Represents the scope of a widget.
pub struct Scope<'a> {
    id: Entity,
    frame: &'a mut Frame,
}

impl<'a> std::fmt::Debug for Scope<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scope").field("id", &self.id).finish()
    }
}

impl<'a> Scope<'a> {
    /// Creates a new scope
    pub(crate) fn spawn(frame: &'a mut Frame) -> Self {
        let mut id = frame.world.spawn();

        Self { id, frame }
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
        F: 'static + FnOnce(&mut Scope<'_>, Fut::Output),
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

    /// Provide a context to all children
    pub fn provide_context<T: ComponentValue>(
        &mut self,
        key: ContextKey<T>,
        value: T,
    ) -> &mut Self {
        self.set(key.into_raw(), value);
        self
    }

    /// Consumes a context provided higher up in the tree.
    pub fn consume_context<T: ComponentValue>(&self, key: ContextKey<T>) -> Option<AtomicRef<T>> {
        let world = &self.frame.world;
        let mut cur = world.entity(self.id).unwrap();
        let key = key.into_raw();
        loop {
            if let Ok(value) = cur.get(key) {
                return Some(value);
            }

            if let Some((parent, _)) = cur.relations(child_of).next() {
                cur = world.entity(parent).unwrap();
            } else {
                return None;
            }
        }
    }

    /// Spawns an effect inside the given scope.
    ///
    /// Returns a handle which will control the effect
    pub fn use_effect<E>(&mut self, effect: E)
    where
        E: 'static + for<'x> Effect<Scope<'x>>,
    {
        // lift App => Scope
        let effect = LiftScope {
            id: self.id,
            effect,
        };

        let handle = self.frame.spawner.spawn(effect);

        // Abort the effect when despawning the entity
        self.entity_mut()
            .entry_ref(tasks())
            .or_default()
            .push(handle);
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

    /// Set a component for the widget
    pub fn set<T: ComponentValue>(&mut self, component: Component<T>, value: T) -> &mut Self {
        self.entity_mut().set(component, value);
        self
    }

    pub fn attach<W: Widget>(&mut self, widget: W) -> Entity {
        let id = self.id;
        let mut child_scope = Scope::spawn(self.frame);
        let child_id = child_scope.id();
        tracing::info!("Attaching {child_id} to {id}");
        child_scope.set(child_of(id), ());

        widget.render(&mut child_scope);
        child_id
    }

    /// Returns the entity id
    pub fn id(&self) -> Entity {
        self.id
    }

    /// Returns the underlying entity for the scope
    pub fn entity(&self) -> EntityRef {
        self.frame
            .world
            .entity(self.id)
            .expect("Entity was despawned")
    }

    /// Returns the underlying entity for the scope
    pub fn entity_mut(&mut self) -> EntityRefMut {
        self.frame
            .world
            .entity_mut(self.id)
            .expect("Entity was despawned")
    }

    pub fn frame_mut(&mut self) -> &mut &'a mut Frame {
        &mut self.frame
    }

    pub fn frame(&self) -> &&'a mut Frame {
        &self.frame
    }
}

/// Lifts a scope local effect to the world.
#[pin_project]
struct LiftScope<E> {
    #[pin]
    effect: E,
    id: Entity,
}

impl<E> Effect<Frame> for LiftScope<E>
where
    E: for<'x> Effect<Scope<'x>>,
{
    fn poll_effect(
        self: std::pin::Pin<&mut Self>,
        frame: &mut Frame,
        cx: &mut Context<'_>,
    ) -> Poll<()> {
        let mut scope = Scope { id: self.id, frame };

        self.project().effect.poll_effect(&mut scope, cx)
    }
}
