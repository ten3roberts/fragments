use std::{
    any::{Any, TypeId},
    collections::BTreeMap,
};

use dashmap::DashMap;
use flax::Component;

use crate::frame::Frame;

pub trait EventHandler<T> {
    /// Handles an event
    ///
    /// Returns `true` if the handler should be kept, `false` if it should be removed
    fn on_event(&mut self, frame: &mut Frame, event: &T) -> bool;
}

impl<F, T> EventHandler<T> for F
where
    F: FnMut(&mut Frame, &T) -> bool,
{
    fn on_event(&mut self, frame: &mut Frame, event: &T) -> bool {
        (self)(frame, event)
    }
}

impl<T: Clone> EventHandler<T> for flume::Sender<T> {
    fn on_event(&mut self, _frame: &mut Frame, event: &T) -> bool {
        self.send(event.clone()).is_ok()
    }
}

/// Stores all the handlers for an event of a specific type
pub struct EventDispatcher<T> {
    handlers: Vec<Box<dyn EventHandler<T>>>,
}

impl<T> std::fmt::Debug for EventDispatcher<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventDispatcher")
            .field("handlers", &self.handlers.len())
            .finish()
    }
}

impl<T> Default for EventDispatcher<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> EventDispatcher<T> {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn register(&mut self, handler: Box<dyn EventHandler<T>>) {
        self.handlers.push(handler);
    }

    /// Emits an event to all handlers
    pub fn emit(&mut self, frame: &mut Frame, event: &T) {
        self.handlers
            .retain_mut(|handler| handler.on_event(frame, event));
    }
}

/// Registry for global events
#[derive(Default, Debug)]
pub struct EventRegistry {
    dispatchers: DashMap<TypeId, Box<dyn Any>>,
}

impl EventRegistry {
    pub fn new() -> Self {
        Default::default()
    }

    /// Subscribe to a global event
    pub fn register<T: 'static>(&self, handler: Box<dyn EventHandler<T>>) {
        self.dispatchers
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(EventDispatcher::<T>::new()))
            .downcast_mut::<EventDispatcher<T>>()
            .unwrap()
            .register(handler);
    }

    /// Emits a global event to all listeners
    pub fn emit<T: 'static>(&self, frame: &mut Frame, event: &T) {
        if let Some(mut dispatcher) = self.dispatchers.get_mut(&TypeId::of::<T>()) {
            dispatcher
                .downcast_mut::<EventDispatcher<T>>()
                .unwrap()
                .emit(frame, event);
        }
    }
}
