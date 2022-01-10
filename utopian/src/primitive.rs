use ash::vk;

use crate::buffer::*;

#[derive(Clone, Debug, Copy)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub color: [f32; 4],
}

pub struct Primitive {
    pub index_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
}

impl Primitive {
    pub fn new(
        device: &ash::Device,
        device_memory_properties: vk::PhysicalDeviceMemoryProperties,
        indices: Vec<u32>,
        vertices: Vec<Vertex>,
    ) -> Primitive {
        let index_buffer = Buffer::new(
            device,
            device_memory_properties,
            indices.as_slice(),
            std::mem::size_of_val(&*indices) as u64,
            vk::BufferUsageFlags::INDEX_BUFFER,
        );

        let vertex_buffer = Buffer::new(
            device,
            device_memory_properties,
            vertices.as_slice(),
            std::mem::size_of_val(&*vertices) as u64,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        );

        Primitive {
            index_buffer,
            vertex_buffer,
            indices,
            vertices,
        }
    }
}
