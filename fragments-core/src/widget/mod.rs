use flax::{Entity, EntityBuilder, World};
pub mod common;

use crate::Scope;

/// A widget is a low level primitive.
///
/// When a widget is rendered it will attach its state and functionality to a node in the UI.
pub trait Widget: BoxedWidget {
    /// Puts the widget into the world in the given scope.
    ///
    /// The scope is used to spawn tasks which update the state.
    fn render(self, scope: &mut Scope);
}

impl<F> Widget for F
where
    F: FnOnce(&mut Scope<'_>),
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

/// Allow calling the consuming widget on a boxed trait object
pub trait BoxedWidget {
    fn render_boxed(self: Box<Self>, scope: &mut Scope);
}

impl<W> BoxedWidget for W
where
    W: Widget,
{
    fn render_boxed(self: Box<Self>, scope: &mut Scope) {
        (*self).render(scope)
    }
}

impl Widget for Box<dyn Widget> {
    fn render(self, scope: &mut Scope) {
        self.render_boxed(scope)
    }
}

/// Represents a list of widgets
trait WidgetCollection {
    fn attach(self, parent: &mut Scope) -> Vec<Entity>;
}

impl WidgetCollection for Vec<Box<dyn Widget>> {
    fn attach(self, parent: &mut Scope) -> Vec<Entity> {
        self.into_iter()
            .map(|widget| parent.attach(widget))
            .collect()
    }
}

impl<const C: usize> WidgetCollection for [Box<dyn Widget>; C] {
    fn attach(self, parent: &mut Scope) -> Vec<Entity> {
        self.map(|widget| parent.attach(widget)).to_vec()
    }
}

impl<W> WidgetCollection for W
where
    W: Widget,
{
    fn attach(self, parent: &mut Scope) -> Vec<Entity> {
        vec![parent.attach(self)]
    }
}

macro_rules! tuple_impl {
    ($($idx: tt => $ty: ident),*) => {
        impl<$($ty),*> WidgetCollection for ($($ty,)*)
            where $($ty: Widget,)*
        {
            fn attach(self, parent: &mut Scope) -> Vec<Entity> {
                vec![ $(
                    parent.attach(self.$idx)
                ),* ]
            }
        }
    };
}

tuple_impl! { 0 => A }
tuple_impl! { 0 => A, 1 => B }
tuple_impl! { 0 => A, 1 => B, 2 => C }
