use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use flax::{name, Entity, World};
use fragments_core::{common::AsyncWidget, Widget};
use glam::Mat4;
use wgpu::{
    BindGroup, BufferUsages, Color, CommandEncoderDescriptor, Operations, RenderPassDescriptor,
    ShaderStages,
};
use winit::window::Window;

use crate::{
    bind_groups::{BindGroupBuilder, BindGroupLayoutBuilder},
    events::{window_size, RedrawEvent, ResizeEvent},
    gpu::Gpu,
    renderer::QuadRenderer,
};

use super::TypedBuffer;

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
        let gpu = AsyncWidget(async move {
            let gpu = Gpu::new(self.window).await;
            tracing::info!("Created gpu");
            GpuView { gpu: Arc::new(gpu) }
        });

        scope.attach(gpu);
        scope.attach(self.root);
    }
}

pub struct GpuView {
    gpu: Arc<Gpu>,
}

impl Widget for GpuView {
    fn mount(self, scope: &mut fragments_core::Scope) {
        scope.set(name(), "GpuView".into());

        let camera = scope.attach(MainCamera {});
        let mut renderer = Renderer::new(&self.gpu, camera);

        scope.on_global_event(move |s, RedrawEvent| {
            renderer.update(&s.frame_mut().world).unwrap();
            renderer.draw().unwrap();
        });

        scope.on_global_event(move |_, &ResizeEvent(new_size)| {
            tracing::info!("Rezing");
            self.gpu.resize(new_size);
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

        scope.set(camera(), Camera { view, proj });

        scope.on_global_event(|s, &ResizeEvent(size)| {
            let view = Mat4::IDENTITY;
            let proj =
                Mat4::orthographic_lh(0.0, size.width as _, size.height as _, 0.0, 0.0, 1000.0);

            s.set(camera(), Camera { view, proj });
        })
    }
}

#[derive(Default, Debug, Clone)]
struct Camera {
    view: Mat4,
    proj: Mat4,
}

flax::component! {
    camera: Camera,
}

#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
struct Globals {
    view: Mat4,
    proj: Mat4,
}

pub struct Renderer {
    camera: Entity,
    gpu: Arc<Gpu>,
    globals_bind_group: BindGroup,
    globals: TypedBuffer<Globals>,
    quad_renderer: QuadRenderer,
}

impl Renderer {
    pub fn new(gpu: &Arc<Gpu>, camera: Entity) -> Self {
        let globals = TypedBuffer::new(
            gpu,
            "globals",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &[Globals {
                view: Mat4::IDENTITY,
                proj: Mat4::IDENTITY,
            }],
        );

        let layout = BindGroupLayoutBuilder::new("globals")
            .bind_uniform_buffer(ShaderStages::VERTEX)
            .build(gpu);

        let globals_bind_group = BindGroupBuilder::new("globals")
            .bind_buffer(&globals)
            .build(gpu, &layout);

        Self {
            camera,
            gpu: gpu.clone(),
            quad_renderer: QuadRenderer::new(gpu, &layout),
            globals_bind_group,
            globals,
        }
    }

    fn update(&mut self, world: &World) -> anyhow::Result<()> {
        if let Ok(camera) = world.get(self.camera, camera()) {
            let globals = Globals {
                view: camera.view,
                proj: camera.proj,
            };

            self.globals.write(&self.gpu.queue, &[globals]);
        } else {
            tracing::error!("No camera");
        }

        self.quad_renderer.update(world);

        Ok(())
    }

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
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
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

            self.quad_renderer
                .draw(&self.globals_bind_group, &mut render_pass)
        }

        gpu.queue.submit([encoder.finish()]);
        surface.present();

        Ok(())
    }
}
