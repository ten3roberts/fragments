use flax::World;

use crate::{effect::TaskSpawner, Scope, Widget};

/// Contains the UI state
///
/// Similar to an Html *Document*
pub struct Frame {
    pub(crate) world: World,
    /// Handle allowing spawning of tasks
    pub(crate) spawner: TaskSpawner<Frame>,
}

impl Frame {
    pub fn new(world: World, spawner: TaskSpawner<Frame>) -> Self {
        Self { world, spawner }
    }

    pub fn spawn_root(&mut self, widget: impl Widget) {
        let mut scope = Scope::spawn(self);
        widget.render(&mut scope);
    }
}
