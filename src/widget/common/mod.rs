use futures::Future;

use crate::Widget;

pub struct AsyncWidget<F>(F);

impl<F> Widget for AsyncWidget<F>
where
    F: 'static + Send + Future,
    F::Output: Widget,
{
    fn render(self, scope: &mut crate::Scope) {
        scope.use_future(self.0, |scope, value| value.render(scope))
    }
}
