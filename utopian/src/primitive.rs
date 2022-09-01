use ash::vk;
use glam::{Vec2, Vec4};
use std::mem;

use crate::buffer::*;
use crate::device::*;
use crate::offset_of;

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
            Some(indices.as_slice()),
            std::mem::size_of_val(&*indices) as u64,
            vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::STORAGE_BUFFER,
            gpu_allocator::MemoryLocation::GpuOnly,
        );

        let vertex_buffer = Buffer::new(
            device,
            Some(vertices.as_slice()),
            std::mem::size_of_val(&*vertices) as u64,
            vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::STORAGE_BUFFER,
            gpu_allocator::MemoryLocation::GpuOnly,
        );

        // Todo: device local index and vertex buffers

        Primitive {
            index_buffer,
            vertex_buffer,
            indices,
            vertices,
        }
    }

    pub fn get_vertex_input_binding_descriptions() -> Vec<vk::VertexInputBindingDescription> {
        [vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
        .to_vec()
    }

    pub fn get_vertex_input_attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, normal) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, uv) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 3,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, color) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 4,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, tangent) as u32,
            },
        ]
        .to_vec()
    }

    pub fn get_vertex_input_create_info() -> vk::PipelineVertexInputStateCreateInfo {
        let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];

        let vertex_input_attribute_descriptions = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, normal) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, uv) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 3,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, color) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 4,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, tangent) as u32,
            },
        ];

        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions)
            .build();

        vertex_input_state_info
    }
}
