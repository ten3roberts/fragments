use flax::*;

use crate::effect::TaskHandle;

component! {
    /// Aborts the stored effects when dropped
    pub(crate) tasks: Vec<TaskHandle<()>>,

    pub text: String => [ flax::Debug ],
    pub resources,
}
