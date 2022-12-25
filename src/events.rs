use flax::{Component, EntityRef};

pub type EventHandler<T> = Box<dyn FnMut(&EntityRef, &T) + Send + Sync>;

/// Send an event to a specific entity
#[inline]
pub fn send_event<T: 'static>(
    entity: EntityRef,
    event_kind: Component<EventHandler<T>>,
    event_data: &T,
) {
    let Ok(mut listener) = entity.get_mut(event_kind) else { return };

    (listener)(&entity, event_data);
}
