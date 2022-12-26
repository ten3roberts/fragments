use flax::*;

use crate::effect::TaskHandle;

component! {
    /// Aborts the stored effects when dropped
    pub(crate) tasks: Vec<TaskHandle>,
    pub(crate) ordered_children: Vec<Entity> => [ Debug ],

    pub text: String => [ Debug ],
    pub resources,
}
