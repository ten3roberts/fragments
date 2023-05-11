use std::sync::Arc;

use flax::name;
use fragments_core::Widget;
use wgpu::{Color, CommandEncoderDescriptor, Operations, RenderPassDescriptor};
use winit::window::Window;

use crate::{
    events::{on_frame, on_resize},
    gpu::{self, Gpu},
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
        scope.set(name(), "GpuProvider".into());
        let gpu = async move {
            let gpu = Gpu::new(self.window).await;
            tracing::info!("Created gpu");
            Arc::new(gpu)
        };

        scope.attach(self.root);
    }
}

pub struct GpuView {
    gpu: Arc<Gpu>,
    renderer: Renderer,
}

impl Widget for GpuView {
    fn mount(self, scope: &mut fragments_core::Scope) {
        scope.set(name(), "GpuView".into());

        let mut renderer = Renderer {
            gpu: self.gpu.clone(),
        };

        scope.set(
            on_frame(),
            Box::new(move |_, _| {
                renderer.draw().unwrap();
            }),
        );

        scope.set(
            on_resize(),
            Box::new(move |_, &new_size| {
                tracing::info!("Rezing");
                self.gpu.resize(new_size);
            }),
        );
    }
}

pub struct Renderer {
    gpu: Arc<Gpu>,
}

impl Renderer {
    fn draw(&mut self) -> anyhow::Result<()> {
        let gpu = &self.gpu;
        let surface = gpu.surface.get_current_texture().unwrap();
        let view = surface.texture.create_view(&Default::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("draw"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: wgpu::LoadOp::Clear(Color {
                            r: 0.2,
                            g: 0.0,
                            b: 0.5,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        }

        gpu.queue.submit([encoder.finish()]);
        surface.present();
        Ok(())
    }
}
