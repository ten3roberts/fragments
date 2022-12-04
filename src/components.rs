use std::sync::{Arc, Weak};

use flax::component;

use crate::signal::Effect;

pub(crate) struct AbortOnDrop {
    effects: Vec<Arc<dyn Effect>>,
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        for effect in &self.effects {
            effect.abort()
        }
    }
}

component! {
    /// Aborts the stored effects when dropped
    pub(crate) abort_on_drop: Vec<Weak<dyn Effect>>,
}
