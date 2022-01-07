use ash::util::*;
use ash::vk;
use std::mem::align_of;

use crate::vulkan_base::*;

pub struct Buffer {
    pub buffer: vk::Buffer,
    pub device_memory: vk::DeviceMemory,
}

impl Buffer {
    pub fn new<T: Copy>(
        device: &ash::Device,
        device_memory_properties: vk::PhysicalDeviceMemoryProperties,
        data: &[T],
        size: u64,
        usage_flags: vk::BufferUsageFlags,
    ) -> Buffer {
        unsafe {
            let buffer_info = vk::BufferCreateInfo::builder()
                .size(size)
                .usage(usage_flags)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let buffer = device
                .create_buffer(&buffer_info, None)
                .expect("Failed to create buffer");

            let buffer_memory_req = device.get_buffer_memory_requirements(buffer);
            let buffer_memory_index = find_memory_type_index(
                &buffer_memory_req,
                &device_memory_properties,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )
            .expect("Unable to find suitable memory type for the buffer");

            let allocate_info = vk::MemoryAllocateInfo {
                allocation_size: buffer_memory_req.size,
                memory_type_index: buffer_memory_index,
                ..Default::default()
            };

            let device_memory = device
                .allocate_memory(&allocate_info, None)
                .expect("Unable to allocate memory for the buffer");

            let buffer_ptr = device
                .map_memory(
                    device_memory,
                    0,
                    buffer_memory_req.size,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("Failed to map buffer memory");

            let mut slice = Align::new(buffer_ptr, align_of::<T>() as u64, buffer_memory_req.size);

            slice.copy_from_slice(&data);

            device.unmap_memory(device_memory);

            device
                .bind_buffer_memory(buffer, device_memory, 0)
                .expect("Failed to bind device memory to buffer");

            println!("size: {}, usage: {:#?}", size, usage_flags);

            Buffer {
                buffer,
                device_memory,
            }
        }
    }
}

