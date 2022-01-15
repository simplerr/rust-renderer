use ash::util::*;
use ash::vk;
use std::mem::align_of;

use crate::device::*;
use crate::image::*;

pub struct Buffer {
    pub buffer: vk::Buffer,
    pub device_memory: vk::DeviceMemory,
    pub size: u64,
}

impl Buffer {
    pub fn new<T: Copy>(
        device: &Device,
        data: &[T],
        size: u64, // todo: get rid of this
        usage_flags: vk::BufferUsageFlags,
    ) -> Buffer {
        unsafe {
            let buffer_info = vk::BufferCreateInfo::builder()
                .size(size)
                .usage(usage_flags)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let buffer = device
                .handle
                .create_buffer(&buffer_info, None)
                .expect("Failed to create buffer");

            let buffer_memory_req = device.handle.get_buffer_memory_requirements(buffer);
            let buffer_memory_index = device
                .find_memory_type_index(
                    &buffer_memory_req,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                )
                .expect("Unable to find suitable memory type for the buffer");

            let allocate_info = vk::MemoryAllocateInfo {
                allocation_size: buffer_memory_req.size,
                memory_type_index: buffer_memory_index,
                ..Default::default()
            };

            let device_memory = device
                .handle
                .allocate_memory(&allocate_info, None)
                .expect("Unable to allocate memory for the buffer");

            let buffer_ptr = device
                .handle
                .map_memory(
                    device_memory,
                    0,
                    buffer_memory_req.size,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("Failed to map buffer memory");

            let mut slice = Align::new(buffer_ptr, align_of::<T>() as u64, buffer_memory_req.size);

            slice.copy_from_slice(&data);

            device.handle.unmap_memory(device_memory);

            device
                .handle
                .bind_buffer_memory(buffer, device_memory, 0)
                .expect("Failed to bind device memory to buffer");

            Buffer {
                buffer,
                device_memory,
                size,
            }
        }
    }

    pub fn copy_to_image(&self, device: &Device, cb: vk::CommandBuffer, image: &Image) {}
}
