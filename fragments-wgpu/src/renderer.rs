use std::sync::Arc;

use bytemuck::Zeroable;
use flax::{Component, Query, World};
use fragments_core::components::{position, size};
use glam::{vec3, Mat4, Vec2, Vec3};
use wgpu::{BufferUsages, RenderPass};

use crate::{
    gpu::Gpu,
    mesh::{Mesh, Vertex},
    typed_buffer::TypedBuffer,
};

pub struct QuadRenderer {
    gpu: Arc<Gpu>,
    quad: Mesh,
    objects: Vec<Object>,
    object_buffer: TypedBuffer<Object>,
    object_query: Query<(Component<Vec2>, Component<Vec2>)>,
}

impl QuadRenderer {
    pub fn new(gpu: &Arc<Gpu>) -> Self {
        const VERTICES: &[Vertex] = &[
            Vertex::new(vec3(0.0, 1.0, 0.0)),
            Vertex::new(vec3(0.5, 1.0, 0.0)),
            Vertex::new(vec3(1.0, 0.0, 0.0)),
            Vertex::new(vec3(0.0, -1.0, 0.0)),
        ];

        const INDICES: &[u32] = &[0, 1, 2, 2, 3, 0];

        let quad = Mesh::new(gpu, VERTICES, INDICES);

        let object_buffer = TypedBuffer::new(
            gpu,
            "object_buffer",
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
            &[Object::zeroed(); 64],
        );

        Self {
            gpu: gpu.clone(),
            quad,
            objects: Vec::new(),
            object_buffer,
            object_query: Query::new((position(), size())),
        }
    }

    pub fn update(&mut self, world: &World) {
        let mut borrow = self.object_query.borrow(world);
        let iter = borrow.iter().map(|(pos, size)| {
            let world_matrix = Mat4::from_scale_rotation_translation(
                size.extend(1.0),
                Default::default(),
                pos.extend(0.1),
            );
            Object { world_matrix }
        });

        self.objects.clear();
        self.objects.extend(iter);

        self.object_buffer.write(&self.gpu.queue, &self.objects);
    }

    pub fn render(&self, render_pass: &mut RenderPass<'_>) {}
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
struct Object {
    world_matrix: Mat4,
}
