use flax::*;
use glam::Vec2;

use crate::effect::TaskHandle;

component! {
    /// Aborts the stored effects when dropped
    pub(crate) tasks: Vec<TaskHandle>,
    pub(crate) ordered_children: Vec<Entity> => [ Debuggable ],

    pub text: String => [ Debuggable ],
    pub resources,

    pub position: Vec2 => [ Debuggable ],
    pub size: Vec2 => [ Debuggable ],
}
