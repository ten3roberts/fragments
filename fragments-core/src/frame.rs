use std::sync::Arc;

use flax::World;

use crate::{effect::TaskSpawner, events::EventRegistry, Scope, Widget};

/// Contains the UI state
///
/// Similar to an Html *Document*
pub struct Frame {
    pub world: World,
    /// Handle allowing spawning of tasks
    pub(crate) spawner: TaskSpawner<Frame>,
    pub events: Arc<EventRegistry>,
}

impl Frame {
    pub fn new(world: World, spawner: TaskSpawner<Frame>, events: Arc<EventRegistry>) -> Self {
        Self {
            world,
            spawner,
            events,
        }
    }

    pub fn spawn_root(&mut self, widget: impl Widget) {
        let mut scope = Scope::spawn(self);
        widget.mount(&mut scope);
    }
}
