use flax::{
    fetch::{entity_refs, EntityRefs},
    Component, Entity, EntityRef, Mutable, Query, World,
};

use crate::components::ordered_children;

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub enum EventState {
    #[default]
    Pending,
    Handled,
}

impl EventState {
    /// Returns `true` if the event state is [`Handled`].
    ///
    /// [`Handled`]: EventState::Handled
    #[must_use]
    fn is_handled(&self) -> bool {
        matches!(self, Self::Handled)
    }
}

pub type EventHandler<T> = Box<dyn 'static + FnMut(EntityRef, &T) + Send + Sync>;

pub struct EventBroadcaster<T: 'static> {
    query: Query<(EntityRefs, Mutable<EventHandler<T>>)>,
}

impl<T> EventBroadcaster<T> {
    pub fn new(event: Component<EventHandler<T>>) -> Self {
        Self {
            query: Query::new((entity_refs(), event.as_mut())),
        }
    }
    /// Broadcast an event to *all* listeners
    pub fn broadcast(&mut self, world: &World, data: &T) {
        for (entity, handler) in &mut self.query.borrow(world) {
            handler(entity, data);
        }
    }
}

///// Send an event down the tree, returning true if the event was handle by any node.
/////
///// Shortcuts for the first entity which could handle the event, in depth-first order.
//#[inline]
//#[tracing::instrument(level = "info", skip(world, event_data))]
//pub fn send_event<T: 'static>(
//    world: &World,
//    id: Entity,
//    event_kind: Component<EventHandler<T>>,
//    event_data: &T,
//) -> EventState {
//    let Ok( entity ) = world.entity(id) else { return EventState::Pending };
//    tracing::info!("Sending events to: {entity:?}");

//    if let Ok(mut listener) = entity.get_mut(event_kind) {
//        let resp = (listener)(entity, event_data);
//        if resp.is_handled() {
//            return EventState::Handled;
//        }
//    }

//    for &id in entity
//        .get(ordered_children())
//        .as_deref()
//        .into_iter()
//        .flatten()
//    {
//        let resp = send_event(world, id, event_kind, event_data);
//        if resp.is_handled() {
//            return resp;
//        }
//    }

//    EventState::Pending
//}
