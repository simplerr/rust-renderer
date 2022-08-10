use ash::util::*;
use ash::vk;
use std::mem::align_of;

use crate::device::*;
use crate::image::*;

pub struct Buffer {
    pub buffer: vk::Buffer,
    pub device_memory: vk::DeviceMemory,
    pub memory_req: vk::MemoryRequirements,
    pub memory_property_flags: vk::MemoryPropertyFlags,
    pub size: u64,
}

impl Buffer {
    pub fn create_buffer(
        device: &Device,
        size: u64, // todo: get rid of this
        usage_flags: vk::BufferUsageFlags,
        memory_property_flags: vk::MemoryPropertyFlags,
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
                .find_memory_type_index(&buffer_memory_req, memory_property_flags)
                .expect("Unable to find suitable memory type for the buffer");

            let mut allocate_info_builder = vk::MemoryAllocateInfo::builder()
                .allocation_size(buffer_memory_req.size)
                .memory_type_index(buffer_memory_index);

            let mut allocate_flags_info = vk::MemoryAllocateFlagsInfo::builder()
                .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS)
                .build();

            if usage_flags.contains(vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS) {
                allocate_info_builder = allocate_info_builder.push_next(&mut allocate_flags_info);
            }

            let allocate_info = allocate_info_builder.build();

            let device_memory = device
                .handle
                .allocate_memory(&allocate_info, None)
                .expect("Unable to allocate memory for the buffer");

            device
                .handle
                .bind_buffer_memory(buffer, device_memory, 0)
                .expect("Failed to bind device memory to buffer");

            Buffer {
                buffer,
                device_memory,
                memory_req: buffer_memory_req,
                memory_property_flags,
                size,
            }
        }
    }

    pub fn new<T: Copy>(
        device: &Device,
        initial_data: Option<&[T]>,
        size: u64, // todo: get rid of this
        usage_flags: vk::BufferUsageFlags,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> Buffer {
        let buffer = Buffer::create_buffer(
            device,
            size,
            usage_flags | vk::BufferUsageFlags::TRANSFER_DST,
            memory_property_flags,
        );

        if let Some(initial_data) = initial_data {
            let staging_buffer = Buffer::create_buffer(
                device,
                size,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            staging_buffer.update_memory(device, initial_data);

            device.execute_and_submit(|device, cb| {
                let regions = vk::BufferCopy::builder()
                    .size(size)
                    .src_offset(0)
                    .dst_offset(0)
                    .build();

                unsafe {
                    device.handle.cmd_copy_buffer(
                        cb,
                        staging_buffer.buffer,
                        buffer.buffer,
                        &[regions],
                    );
                }
            });

            unsafe {
                device
                    .handle
                    .free_memory(staging_buffer.device_memory, None);
                device.handle.destroy_buffer(staging_buffer.buffer, None);
            }
        }

        buffer
    }

    pub fn update_memory<T: Copy>(&self, device: &Device, data: &[T]) {
        unsafe {
            if self.memory_property_flags != vk::MemoryPropertyFlags::DEVICE_LOCAL {
                let buffer_ptr = device
                    .handle
                    .map_memory(
                        self.device_memory,
                        0,
                        self.memory_req.size,
                        vk::MemoryMapFlags::empty(),
                    )
                    .expect("Failed to map buffer memory");

                let mut slice =
                    Align::new(buffer_ptr, align_of::<T>() as u64, self.memory_req.size);

                slice.copy_from_slice(data);

                device.handle.unmap_memory(self.device_memory);
            } else {
                let staging_buffer = Buffer::create_buffer(
                    device,
                    self.size,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                );

                let buffer_ptr = device
                    .handle
                    .map_memory(
                        staging_buffer.device_memory,
                        0,
                        self.memory_req.size,
                        vk::MemoryMapFlags::empty(),
                    )
                    .expect("Failed to map buffer memory");

                let mut slice =
                    Align::new(buffer_ptr, align_of::<T>() as u64, self.memory_req.size);

                slice.copy_from_slice(data);

                device.handle.unmap_memory(staging_buffer.device_memory);

                device.execute_and_submit(|device, cb| {
                    let regions = vk::BufferCopy::builder()
                        .size(self.size)
                        .src_offset(0)
                        .dst_offset(0)
                        .build();

                    device.handle.cmd_copy_buffer(
                        cb,
                        staging_buffer.buffer,
                        self.buffer,
                        &[regions],
                    );
                });

                device
                    .handle
                    .free_memory(staging_buffer.device_memory, None);
                device.handle.destroy_buffer(staging_buffer.buffer, None);
            }
        }
    }

    pub fn copy_to_image(&self, device: &Device, cb: vk::CommandBuffer, image: &Image) {
        let buffer_copy_regions = vk::BufferImageCopy::builder()
            .image_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1)
                    .build(),
            )
            .image_extent(vk::Extent3D {
                width: image.width,
                height: image.height,
                depth: 1,
            });

        unsafe {
            device.handle.cmd_copy_buffer_to_image(
                cb,
                self.buffer,
                image.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[buffer_copy_regions.build()],
            );
        }
    }

    pub fn get_device_address(&self, device: &Device) -> vk::DeviceAddress {
        let info = vk::BufferDeviceAddressInfo::builder()
            .buffer(self.buffer)
            .build();

        unsafe { device.handle.get_buffer_device_address(&info) }
    }
}
