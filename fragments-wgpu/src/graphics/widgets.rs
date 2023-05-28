use std::sync::Arc;

use flax::name;
use fragments_core::Widget;
use glam::Mat4;
use winit::window::Window;

use crate::{
    events::{window_size, RedrawEvent, ResizeEvent},
    gpu::Gpu,
    renderer::Renderer,
};

pub(crate) struct GpuProvider<W> {
    window: Window,
    root: W,
}

impl<W> GpuProvider<W> {
    pub(crate) fn new(window: Window, root: W) -> Self {
        Self { window, root }
    }
}

impl<W> Widget for GpuProvider<W>
where
    W: 'static + Widget,
{
    fn mount(self, scope: &mut fragments_core::Scope) {
        scope.provide_context(window_size(), self.window.inner_size());

        scope.set(name(), "GpuProvider".into());

        let root = self.root;
        scope.use_async(Gpu::new(self.window), move |scope, gpu| {
            let gpu = Arc::new(gpu);
            let camera = scope.attach(MainCamera {});
            let mut renderer = Renderer::new(&gpu, camera);

            scope.on_global_event(move |s, RedrawEvent| {
                renderer.update(&mut s.frame_mut().world).unwrap();
                renderer.draw().unwrap();
            });

            scope.on_global_event(move |_, &ResizeEvent(new_size)| {
                tracing::info!("Rezing");
                gpu.resize(new_size);
            });

            scope.attach(root);
        });
    }
}

struct MainCamera {}

impl Widget for MainCamera {
    fn mount(self, scope: &mut fragments_core::Scope<'_>) {
        scope.set(name(), "MainCamera".into());

        let size = *scope
            .consume_context(window_size())
            .expect("No window size");

        let view = Mat4::IDENTITY;
        let proj = Mat4::orthographic_lh(0.0, size.width as _, size.height as _, 0.0, 0.0, 1000.0);

        scope.set(view_matrix(), view);
        scope.set(proj_matrix(), proj);

        scope.on_global_event(|s, &ResizeEvent(size)| {
            let proj =
                Mat4::orthographic_lh(0.0, size.width as _, size.height as _, 0.0, 0.0, 1000.0);
            s.set(proj_matrix(), proj);
        })
    }
}

flax::component! {
    pub view_matrix: Mat4,
    pub proj_matrix: Mat4,
    pub scale_to_window: (),
}
