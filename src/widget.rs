use flax::{Entity, EntityBuilder, World};

use crate::Scope;

pub trait Widget {
    fn render(self, scope: Scope);
}

impl<F> Widget for F
where
    F: FnMut(Scope<'_>),
{
    fn render(mut self, scope: Scope) {
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
