use futures::Future;

use crate::{components::ordered_children, Widget};

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
