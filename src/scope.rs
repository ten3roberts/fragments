use flax::{child_of, Component, ComponentValue, Entity, EntityRefMut, World};

use crate::{
    components::tasks,
    effect::{Effect, EffectSender, SignalEffect, Task},
    signal::Signal,
    App, Widget,
};

/// Represents the scope of a widget.
pub struct Scope<'a> {
    entity: EntityRefMut<'a>,
    effects_tx: &'a EffectSender,
}

impl<'a> Scope<'a> {
    /// Creates a new scope
    pub(crate) fn spawn(
        world: &'a mut World,
        effects_tx: &'a EffectSender,
        parent: Option<Entity>,
    ) -> Self {
        let mut entity = world.spawn_ref();
        if let Some(parent) = parent {
            entity.set(child_of(parent), ());
        }

        Self { entity, effects_tx }
    }

    /// Executes `func` whenever the signal value changes
    pub fn use_signal<S, T, F>(&mut self, signal: S, mut func: F)
    where
        S: 'static + Send + Sync + for<'x> Signal<'x, Item = T>,
        F: 'static + FnMut(Scope<'_>, T) + Send + Sync,
    {
        let id = self.entity.id();
        let effect = SignalEffect::new(signal, move |app: &mut App, item: S::Item| {
            if let Some(s) = Scope::reconstruct(&mut app.world, &app.effects_tx, id) {
                func(s, item);
            }
        });

        self.spawn_effect(effect);
    }

    /// Spawns the effect into the app.
    ///
    /// Returns a handle which will control the effect
    fn spawn_effect<E: 'static + Effect<Output = ()> + Send>(&mut self, effect: E) {
        let (task, handle) = Task::new(Box::pin(effect), self.effects_tx.clone());

        // Abort the effect when despawning the entity
        self.entity.entry_ref(tasks()).or_default().push(handle);

        self.effects_tx.send(task).ok();
    }

    /// Reconstruct the scope for an entity
    fn reconstruct(world: &'a mut World, effects_tx: &'a EffectSender, id: Entity) -> Option<Self> {
        let entity = world.entity_mut(id).ok()?;

        Some(Self { entity, effects_tx })
    }

    /// Set a component for the widget
    pub fn set<T: ComponentValue>(&mut self, component: Component<T>, value: T) -> &mut Self {
        self.entity.set(component, value);
        self
    }

    pub fn attach_child<W: Widget>(&mut self, widget: W) {
        let id = self.entity.id();
        let child_scope = Scope::spawn(self.entity.world_mut(), self.effects_tx, Some(id));

        widget.render(child_scope);
    }
}
