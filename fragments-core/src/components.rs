use flax::*;
use palette::Srgba;

use crate::effect::TaskHandle;

#[derive(Default)]
pub(crate) struct OnCleanup(Vec<Box<dyn FnOnce() + Send + Sync>>);

impl std::fmt::Debug for OnCleanup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OnCleanup").field(&self.0.len()).finish()
    }
}

impl OnCleanup {
    pub fn push(&mut self, func: Box<dyn FnOnce() + Send + Sync>) {
        self.0.push(func);
    }
}

impl Drop for OnCleanup {
    fn drop(&mut self) {
        self.0.drain(..).for_each(|func| func());
    }
}

component! {
    /// Aborts the stored effects when dropped
    pub(crate) tasks: Vec<TaskHandle>,
    pub(crate) ordered_children: Vec<Entity> => [ Debuggable ],
    pub(crate) on_cleanup: OnCleanup => [ Debuggable ],
    /// Runs when a widget is unmounted/detached

    pub text: String => [ Debuggable ],

    pub color: Srgba => [ Debuggable ],

    pub resources,
}
