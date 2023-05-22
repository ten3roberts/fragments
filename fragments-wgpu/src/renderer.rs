use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use flax::{Entity, Query, World};
use glam::Mat4;
use wgpu::{
    BindGroup, BufferUsages, Color, CommandEncoderDescriptor, Operations, RenderPassDescriptor,
    ShaderStages,
};

use crate::{
    bind_groups::{BindGroupBuilder, BindGroupLayoutBuilder},
    gpu::Gpu,
    graphics::{proj_matrix, view_matrix, TypedBuffer},
    quad_renderer::QuadRenderer,
};

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

    pub fn update(&mut self, world: &mut World) -> anyhow::Result<()> {
        if let Ok((&view, &proj)) = Query::new((view_matrix(), proj_matrix()))
            .borrow(&world)
            .get(self.camera)
        {
            let globals = Globals { view, proj };

            self.globals.write(&self.gpu.queue, &[globals]);
        } else {
            tracing::error!("No camera");
        }

        self.quad_renderer.update(world);

        Ok(())
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
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
