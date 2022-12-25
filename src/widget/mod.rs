use flax::{Entity, EntityBuilder, World};
pub mod common;

use crate::Scope;

pub trait Widget {
    fn render(self, scope: &mut Scope);
}

impl<F> Widget for F
where
    F: FnMut(&mut Scope<'_>),
{
    fn render(mut self, scope: &mut Scope) {
        (self)(scope)
    }
}

pub struct Fragment {
    data: EntityBuilder,
}

impl Fragment {
    pub fn spawn(mut self, world: &mut World) -> Entity {
        self.data.spawn(world)
    }
}
