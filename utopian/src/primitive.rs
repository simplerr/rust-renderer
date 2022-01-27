use ash::vk;
use glam::{Vec2, Vec3, Vec4};

use crate::buffer::*;
use crate::device::*;

#[derive(Clone, Debug, Copy)]
pub struct Vertex {
    pub pos: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub color: Vec4,
    pub tangent: Vec4,
}

pub struct Primitive {
    pub index_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
}

impl Primitive {
    pub fn new(device: &Device, indices: Vec<u32>, vertices: Vec<Vertex>) -> Primitive {
        let index_buffer = Buffer::new(
            device,
            indices.as_slice(),
            std::mem::size_of_val(&*indices) as u64,
            vk::BufferUsageFlags::INDEX_BUFFER,
        );

        let vertex_buffer = Buffer::new(
            device,
            vertices.as_slice(),
            std::mem::size_of_val(&*vertices) as u64,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        );

        // Todo: device local index and vertex buffers

        Primitive {
            index_buffer,
            vertex_buffer,
            indices,
            vertices,
        }
    }
}
