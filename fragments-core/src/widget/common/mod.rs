use std::any::type_name;

use flax::name;
use futures::Future;

use crate::{components::ordered_children, signal::Signal, Widget};

use super::WidgetCollection;

pub struct AsyncWidget<F>(F);

impl<F> Widget for AsyncWidget<F>
where
    F: 'static + Future,
    F::Output: Widget,
{
    fn render(self, scope: &mut crate::Scope) {
        scope.use_future(self.0, |scope, value| value.render(scope))
    }
}

pub struct Container<W>(pub W);

impl<W> Widget for Container<W>
where
    W: WidgetCollection,
{
    fn render(self, scope: &mut crate::Scope) {
        let ids = self.0.attach(scope);

        scope.set(ordered_children(), ids);
    }
}

impl<S, W> Widget for S
where
    S: 'static + for<'x> Signal<'x, Item = W>,
    W: 'static + Widget,
{
    fn render(self, scope: &mut crate::Scope) {
        let mut child = None;

        scope.use_signal(self, move |scope, item| {
            if let Some(id) = child.take() {
                scope.detach(id);
            }

            let id = scope.attach(item);
            child = Some(id);
        });

        scope.set(name(), type_name::<Self>().into());
    }
}
