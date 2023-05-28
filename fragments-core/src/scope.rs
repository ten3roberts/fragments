use std::{
    marker::PhantomData,
    task::{Context, Poll},
};

use atomic_refcell::AtomicRef;
use flax::{
    archetype::RefMut, child_of, name, Component, ComponentValue, Entity, EntityBuilder, EntityRef,
    EntityRefMut, World,
};
use futures::{Future, SinkExt, Stream};
use pin_project::pin_project;

use crate::{
    components::{on_cleanup, ordered_children, tasks},
    context::ContextKey,
    effect::{Effect, FutureEffect, SignalEffect, StreamEffect, TaskSpawner},
    events::EventHandler,
    frame::Frame,
    signal::Signal,
    Widget,
};

/// Context for the given widget, allows for spawning tasks, attaching components and children
pub struct Scope<'a> {
    frame: &'a mut Frame,
    id: Entity,
    data: EntityBuilder,
}

impl<'a> std::fmt::Debug for Scope<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scope").field("id", &self.id).finish()
    }
}

impl<'a> Scope<'a> {
    /// Spawns a new scope
    pub fn spawn(frame: &'a mut Frame) -> Self {
        let id = frame.world.spawn();

        Self {
            frame,
            id,
            data: Entity::builder(),
        }
    }

    /// Recreates a scope from an existing entity if it exists
    pub fn try_from_id(frame: &'a mut Frame, id: Entity) -> Option<Self> {
        if !frame.world.is_alive(id) {
            return None;
        }
        Some(Self {
            frame,
            id,
            data: Entity::builder(),
        })
    }

    /// Creates a new effect to be run in this scope
    ///
    /// The effect will be stopped when the widget is unmounted
    pub fn create_effect<E>(&mut self, effect: E)
    where
        E: 'static + for<'x> Effect<Scope<'x>>,
    {
        let effect = Lift {
            effect,
            id: self.id,
        };

        let handle = self.frame.spawner.spawn(effect);
        self.on_cleanup(move || handle.abort());

        // pub fn on_cleanup(&mut self, func: impl 'static + FnOnce(&mut Scope)) {
        self.flush();
    }

    /// Executes `func` within the scope of the widget when the signal is emitted
    pub fn use_signal<S, F, T>(&mut self, signal: S, func: F)
    where
        S: 'static + for<'x> Signal<'x, Item = T>,
        F: 'static + for<'x> FnMut(&'x mut Scope<'_>, T),
    {
        self.create_effect(SignalEffect::new(signal, func))
    }

    /// Executes `func` within the scope of the widget when the async future completes.
    pub fn use_async<T>(
        &mut self,
        future: impl 'static + Future<Output = T>,
        func: impl 'static + FnOnce(&mut Scope, T),
    ) {
        self.create_effect(FutureEffect::new(future, func))
    }

    pub fn on_cleanup(&mut self, func: impl 'static + Send + Sync + FnOnce()) {
        self.flush();
        self.frame
            .world
            .entry(self.id, on_cleanup())
            .unwrap()
            .or_default()
            .push(Box::new(func));
    }

    /// Write the changes to the world
    fn flush(&mut self) {
        tracing::debug!("Flushing scope: {:?}", self.data);
        self.data
            .append_to(&mut self.frame.world, self.id)
            .expect("Invalid entity");
    }
}

impl<'a> Scope<'a> {
    /// Sets the component for the widget
    pub fn set<T: ComponentValue>(&mut self, component: Component<T>, value: T) {
        self.data.set(component, value);
    }

    /// Sets the component for the widget using the default value
    pub fn set_default<T: ComponentValue + Default>(&mut self, component: Component<T>) {
        self.data.set_default(component);
    }

    pub fn remove<T: ComponentValue>(&mut self, component: Component<T>) {
        self.data.remove(component);
    }

    /// Mounts a widget as un **unordered** child of the current scope.
    ///
    /// Returns the children entity id, which can be used to enforce ordering.
    pub fn attach<W: Widget>(&mut self, widget: W) -> Entity {
        self.flush();

        let mut child = Scope::spawn(self.frame);

        child.set(name(), tynm::type_name::<W>());
        child.set(child_of(self.id), ());
        child.flush();

        widget.mount(&mut child);
        let id = child.id;

        drop(child);

        self.entity_mut()
            .entry(ordered_children())
            .or_default()
            .push(id);

        self.flush();

        id
    }

    /// Detaches a child from the current scope
    pub fn detach(&mut self, id: Entity) {
        assert!(
            self.frame.world.has(id, child_of(self.id)),
            "Attempt to despawn a widget {id} that is not a child of the current scope {}",
            self.id
        );

        self.frame.world.despawn_recursive(id, child_of).unwrap();
    }
}

impl<'a> Drop for Scope<'a> {
    fn drop(&mut self) {
        self.flush()
    }
}

impl<'a> Scope<'a> {
    // pub fn use_signal<S, F, T>(&mut self, signal: S, func: F)
    // where
    //     S: 'static + for<'x> Signal<'x, Item = T>,
    //     F: 'static + FnMut(&mut Scope<'_>, T),
    // {
    //     self.use_effect(SignalEffect::new(signal, func))
    // }

    // pub fn use_future<Fut, F>(&mut self, fut: Fut, func: F)
    // where
    //     Fut: 'static + Future,
    //     F: 'static + FnOnce(&mut Scope<'_>, Fut::Output),
    // {
    //     self.use_effect(FutureEffect::new(fut, func))
    // }

    // pub fn use_stream<S, F>(&mut self, fut: S, func: F)
    // where
    //     S: 'static + Stream,
    //     F: 'static + FnMut(&mut Scope<'_>, S::Item),
    // {
    //     self.use_effect(StreamEffect::new(fut, func))
    // }

    /// React to globally emitted events
    pub fn on_global_event<T: 'static>(
        &mut self,
        handler: impl 'static + FnMut(&mut Scope<'_>, &T),
    ) {
        self.frame
            .events
            .register::<T>(Box::new(ScopedEventHandler {
                handler,
                id: self.id,
                _marker: PhantomData,
            }));
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
        tracing::info!(?self.id, "World:  {world:#?}");
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

    ///// Spawns an effect inside the given scope.
    /////
    ///// Returns a handle which will control the effect
    //pub fn use_effect<E>(&mut self, effect: E)
    //where
    //    E: 'static + for<'x> Effect<Scope<'x>>,
    //{
    //    // lift App => Scope
    //    let effect = LiftScope {
    //        id: self.id,
    //        effect,
    //    };

    //    let handle = self.frame.spawner.spawn(effect);

    //    // Abort the effect when despawning the entity
    //    self.entity_mut()
    //        .entry_ref(tasks())
    //        .or_default()
    //        .push(handle);
    //}

    ///// Listener for a global event
    /////
    ///// The event handlers run without mutable access to the world, and can as such not attach new
    ///// children.
    //pub fn on_global_event<E>(
    //    &mut self,
    //    event_kind: Component<EventHandler<E>>,
    //    event_handler: impl 'static + Send + Sync + FnMut(EntityRef, &E),
    //) -> &mut Self
    //where
    //    E: 'static,
    //{
    //    self.set(event_kind, Box::new(event_handler));
    //    self
    //}

    ///// Set a component for the widget
    //pub fn set<T: ComponentValue>(&mut self, component: Component<T>, value: T) -> &mut Self {
    //    self.entity_mut().set(component, value);
    //    self
    //}

    //pub fn attach<W: Widget>(&mut self, widget: W) -> Entity {
    //    let id = self.id;
    //    let mut child_scope = Scope::spawn(self.frame);
    //    let child_id = child_scope.id();
    //    child_scope.set(child_of(id), ());

    //    widget.mount(&mut child_scope);
    //    child_id
    //}

    ///// Returns the entity id
    //pub fn id(&self) -> Entity {
    //    self.id
    //}

    /// Returns the underlying entity for the scope
    fn entity(&self) -> EntityRef {
        assert_eq!(self.data.component_count(), 0);
        self.frame
            .world
            .entity(self.id)
            .expect("Entity was despawned")
    }

    /// Returns the underlying entity for the scope
    fn entity_mut(&mut self) -> EntityRefMut {
        assert_eq!(self.data.component_count(), 0);
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

    // /// Detaches a child from the current scope
    // pub fn detach(&mut self, id: Entity) {
    //     self.frame.world.despawn_recursive(id, child_of).unwrap();
    // }
}

/// Lifts a scope local effect to the world.
#[pin_project]
struct Lift<E> {
    #[pin]
    effect: E,
    id: Entity,
}

impl<E> Effect<Frame> for Lift<E>
where
    E: for<'x> Effect<Scope<'x>>,
{
    fn poll_effect(
        self: std::pin::Pin<&mut Self>,
        frame: &mut Frame,
        cx: &mut Context<'_>,
    ) -> Poll<()> {
        let p = self.project();

        match Scope::try_from_id(frame, *p.id) {
            Some(mut v) => p.effect.poll_effect(&mut v, cx),
            None => {
                tracing::info!("Scope was despawned, aborting effect");
                Poll::Ready(())
            }
        }
    }
}

struct ScopedEventHandler<F, T> {
    handler: F,
    id: Entity,
    _marker: PhantomData<T>,
}

impl<F, T> EventHandler<T> for ScopedEventHandler<F, T>
where
    F: FnMut(&mut Scope<'_>, &T),
{
    fn on_event(&mut self, frame: &mut Frame, event: &T) -> bool {
        match Scope::try_from_id(frame, self.id) {
            Some(mut scope) => {
                (self.handler)(&mut scope, event);
                true
            }
            None => false,
        }
    }
}
