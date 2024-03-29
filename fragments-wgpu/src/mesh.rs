use glam::{vec2, vec3, Vec2, Vec3};
use wgpu::{
    util::DeviceExt, vertex_attr_array, Buffer, RenderPass, VertexAttribute, VertexBufferLayout,
};

use crate::gpu::Gpu;

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Debug, Clone)]
pub struct Vertex {
    pos: Vec3,
}

pub trait VertexDesc {
    fn layout() -> VertexBufferLayout<'static>;
}

impl Vertex {
    pub const fn new(pos: Vec3) -> Self {
        Self { pos }
    }
}
impl VertexDesc for Vertex {
    fn layout() -> VertexBufferLayout<'static> {
        static ATTRIBUTES: &[VertexAttribute] = &vertex_attr_array![0 => Float32x3];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

pub struct Mesh {
    vertex_count: u32,
    index_count: u32,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl Mesh {
    pub fn new(gpu: &Gpu, vertices: &[Vertex], indices: &[u32]) -> Self {
        let vertex_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Self {
            vertex_count: vertices.len() as u32,
            index_count: indices.len() as u32,
            vertex_buffer,
            index_buffer,
        }
    }

    /// Creates a new mesh with dimensions 1x1
    pub fn square(gpu: &Gpu) -> Self {
        let vertices = [
            Vertex::new(vec3(0.0, 0.0, 0.0)),
            Vertex::new(vec3(1.0, 0.0, 0.0)),
            Vertex::new(vec3(1.0, 1.0, 0.0)),
            Vertex::new(vec3(0.0, 1.0, 0.0)),
        ];

        let indices = [0, 1, 2, 2, 3, 0];

        Self::new(gpu, &vertices, &indices)
    }

    pub fn bind<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    }

    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }
}
