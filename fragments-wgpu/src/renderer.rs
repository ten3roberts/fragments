use std::{borrow::Cow, sync::Arc};

use bytemuck::Zeroable;
use flax::{Component, Fetch, Query, World};
use fragments_core::layout::{position, size};
use glam::{vec3, Mat4, Vec2};
use wgpu::{BindGroup, BindGroupLayout, BufferUsages, RenderPass, ShaderStages};

use crate::{
    bind_groups::{BindGroupBuilder, BindGroupLayoutBuilder},
    gpu::Gpu,
    graphics::{
        shader::{Shader, ShaderDesc},
        TypedBuffer,
    },
    mesh::{Mesh, Vertex, VertexDesc},
};

pub struct QuadRenderer {
    gpu: Arc<Gpu>,
    quad: Mesh,
    objects: Vec<Object>,
    object_buffer: TypedBuffer<Object>,
    object_bind_group: BindGroup,
    object_query: Query<ObjectQuery>,
    shader: Shader,
}

impl QuadRenderer {
    pub fn new(gpu: &Arc<Gpu>, globals_layout: &BindGroupLayout) -> Self {
        const VERTICES: &[Vertex] = &[
            Vertex::new(vec3(-0.5, -0.5, 0.0)),
            Vertex::new(vec3(0.5, -0.5, 0.0)),
            Vertex::new(vec3(0.5, 0.5, 0.0)),
            Vertex::new(vec3(-0.5, 0.5, 0.0)),
        ];

        const INDICES: &[u32] = &[0, 1, 2, 2, 3, 0];

        let quad = Mesh::new(gpu, VERTICES, INDICES);

        let object_buffer = TypedBuffer::new(
            gpu,
            "object_buffer",
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
            &[Object::zeroed(); 64],
        );

        let object_layout = BindGroupLayoutBuilder::new("quad_renderer")
            .bind_storage_buffer(ShaderStages::VERTEX)
            .build(gpu);

        let object_bind_group = BindGroupBuilder::new("quad_renderer")
            .bind_buffer(&object_buffer)
            .build(gpu, &object_layout);

        let shader = Shader::new(
            gpu,
            ShaderDesc {
                label: "quad_renderer",
                source: include_str!("../assets/solid.wgsl").into(),
                format: gpu.surface_format(),
                vertex_layouts: Cow::Borrowed(&[Vertex::layout()]),
                layouts: &[globals_layout, &object_layout],
            },
        );

        Self {
            gpu: gpu.clone(),
            quad,
            objects: Vec::new(),
            object_bind_group,
            object_buffer,
            object_query: Query::new(ObjectQuery::new()),
            shader,
        }
    }

    pub fn update(&mut self, world: &World) {
        let mut borrow = self.object_query.borrow(world);
        let iter = borrow.iter().map(|q| {
            let world_matrix = Mat4::from_scale_rotation_translation(
                q.size.extend(1.0),
                Default::default(),
                q.pos.extend(0.1),
            );
            Object { world_matrix }
        });

        self.objects.clear();
        self.objects.extend(iter);

        self.object_buffer.write(&self.gpu.queue, &self.objects);
    }

    pub fn draw<'a>(&'a self, global_bind_groups: &'a BindGroup, render_pass: &mut RenderPass<'a>) {
        render_pass.set_pipeline(self.shader.pipeline());

        for (i, bind_group) in [global_bind_groups, &self.object_bind_group]
            .iter()
            .enumerate()
        {
            render_pass.set_bind_group(i as _, bind_group, &[]);
        }

        self.quad.bind(render_pass);

        render_pass.draw_indexed(0..6, 0, 0..self.objects.len() as _);
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
struct Object {
    world_matrix: Mat4,
}

#[derive(Debug, Fetch)]
struct ObjectQuery {
    size: Component<Vec2>,
    pos: Component<Vec2>,
}

impl ObjectQuery {
    fn new() -> Self {
        Self {
            size: size(),
            pos: position(),
        }
    }
}
