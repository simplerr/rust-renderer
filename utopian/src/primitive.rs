use ash::vk;
use glam::{Vec2, Vec4};

use crate::buffer::*;
use crate::device::*;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Vertex {
    pub pos: Vec4,
    pub normal: Vec4,
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

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Vertex {
        Vertex {
            pos: Vec4::new(x, y, z, 0.0),
            normal: Vec4::new(0.0, 0.0, 0.0, 0.0),
            uv: Vec2::new(0.0, 0.0),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            tangent: Vec4::new(0.0, 0.0, 0.0, 0.0),
        }
    }
}

impl Primitive {
    pub fn new(device: &Device, indices: Vec<u32>, vertices: Vec<Vertex>) -> Primitive {
        let index_buffer = Buffer::new(
            device,
            indices.as_slice(),
            std::mem::size_of_val(&*indices) as u64,
            vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::STORAGE_BUFFER,
        );

        let vertex_buffer = Buffer::new(
            device,
            vertices.as_slice(),
            std::mem::size_of_val(&*vertices) as u64,
            vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::STORAGE_BUFFER,
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
