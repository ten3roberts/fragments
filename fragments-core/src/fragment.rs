use std::{
    pin::Pin,
    task::{Context, Poll},
};

use flax::{Entity, EntityBuilder};

use crate::{effect::Effect, frame::Frame};

/// Represents the output of the widget; a part in the graph
pub struct Fragment {
    id: Entity,
    entity: EntityBuilder,
    /// Effects lifted to the spawned entity
    effects: Vec<Pin<Box<dyn Effect<Frame>>>>,
}

impl Fragment {
    pub(crate) fn spawn_root(mut self, frame: &mut Frame) -> Entity {
        self.spawn_inner(frame, None)
    }

    pub fn spawn(mut self, frame: &mut Frame, parent: Entity) -> Entity {
        self.spawn_inner(frame, Some(parent))
    }

    pub(crate) fn spawn_inner(mut self, frame: &mut Frame, parent: Option<Entity>) -> Entity {
        self.entity.spawn_at(&mut frame.world, self.id);

        for effect in self.effects {
            frame.spawner.spawn_boxed(effect);
        }

        self.id
    }
}

impl Fragment {
    pub fn create_effect<E>(&mut self, effect: E) -> &mut Self
    where
        E: 'static + for<'x> Effect<Scope<'x>>,
    {
        self.effects.push(Box::pin(Lift {
            id: self.id,
            effect,
        }));
        self
    }
}

#[pin_project::pin_project]
pub struct Lift<E> {
    id: Entity,
    #[pin]
    effect: E,
}

impl<E> Effect<Frame> for Lift<E>
where
    E: for<'x> Effect<Scope<'x>>,
{
    fn poll_effect(self: Pin<&mut Self>, frame: &mut Frame, cx: &mut Context<'_>) -> Poll<()> {
        let mut p = self.project();
        if frame.world.is_alive(*p.id) {
            let mut scope = Scope { id: *p.id, frame };
            p.effect.poll_effect(&mut scope, cx)
        } else {
            Poll::Ready(())
        }
    }
}

pub struct Scope<'a> {
    frame: &'a mut Frame,
    id: Entity,
}
