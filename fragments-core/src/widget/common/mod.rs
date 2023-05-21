use crate::{
    components::ordered_children,
    effect::{FutureEffect, StreamEffect},
    signal::Signal,
    Scope, Widget,
};
use flax::name;
use futures::{Future, Stream};

use super::WidgetCollection;

pub struct AsyncWidget<F>(pub F);

impl<F> Widget for AsyncWidget<F>
where
    F: 'static + Future,
    F::Output: Widget,
{
    fn mount(self, scope: &mut crate::Scope) {
        scope.create_effect(FutureEffect::new(
            self.0,
            |scope: &mut Scope<'_>, value: F::Output| value.mount(scope),
        ))
    }
}

pub struct Container<W>(pub W);

impl<W> Widget for Container<W>
where
    W: WidgetCollection,
{
    fn mount(self, scope: &mut crate::Scope) {
        let ids = self.0.attach(scope);

        scope.set(ordered_children(), ids);
    }
}

impl<S, W> Widget for S
where
    S: 'static + for<'x> Signal<'x, Item = W>,
    W: 'static + Widget,
{
    fn mount(self, scope: &mut crate::Scope) {
        let mut child = None;

        scope.use_signal(self, move |scope, item| {
            if let Some(id) = child.take() {
                scope.detach(id);
            }

            let id = scope.attach(item);
            child = Some(id);
        });

        scope.set(name(), tynm::type_name::<Self>());
    }
}

pub struct Streaming<S>(pub S);

impl<S> Widget for Streaming<S>
where
    S: 'static + Stream,
    S::Item: Widget,
{
    fn mount(self, scope: &mut crate::Scope) {
        let mut child = None;

        scope.create_effect(StreamEffect::new(self.0, move |scope: &mut Scope, item| {
            if let Some(id) = child.take() {
                scope.detach(id);
            }

            let id = scope.attach(item);
            child = Some(id);
        }));

        scope.set(name(), tynm::type_name::<Self>());
    }
}
