use ash::vk;
use gpu_allocator::vulkan::*;

use crate::device::*;
use crate::image::*;

pub struct Buffer {
    pub buffer: vk::Buffer,
    pub allocation: Allocation,
    pub memory_req: vk::MemoryRequirements,
    pub memory_location: gpu_allocator::MemoryLocation,
    pub size: u64,
    pub debug_name: String,
}

impl Buffer {
    pub fn create_buffer(
        device: &Device,
        size: u64, // todo: get rid of this
        usage_flags: vk::BufferUsageFlags,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Buffer {
        puffin::profile_function!();

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

            let allocation = device
                .gpu_allocator
                .lock()
                .unwrap()
                .allocate(&AllocationCreateDesc {
                    name: "Example allocation",
                    requirements: buffer_memory_req,
                    location: memory_location,
                    linear: true,
                })
                .unwrap();

            device
                .handle
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .unwrap();

            Buffer {
                buffer,
                allocation,
                memory_req: buffer_memory_req,
                memory_location,
                size,
                debug_name: String::from("unnamed_buffer"),
            }
        }
    }

    pub fn new<T: Copy>(
        device: &Device,
        initial_data: Option<&[T]>,
        size: u64, // todo: get rid of this
        usage_flags: vk::BufferUsageFlags,
        location: gpu_allocator::MemoryLocation,
    ) -> Buffer {
        let mut buffer = Buffer::create_buffer(
            device,
            size,
            usage_flags | vk::BufferUsageFlags::TRANSFER_DST,
            location,
        );

        if let Some(initial_data) = initial_data {
            buffer.update_memory(device, initial_data);
        }

        buffer.set_debug_name(device, "unnamed_buffer");

        buffer
    }

    pub fn update_memory<T: Copy>(&mut self, device: &Device, data: &[T]) {
        unsafe {
            let src = data.as_ptr() as *const u8;
            let src_bytes = data.len() * std::mem::size_of::<T>();

            if self.memory_location != gpu_allocator::MemoryLocation::GpuOnly {
                let dst = self.allocation.mapped_ptr().unwrap().as_ptr() as *mut u8;
                let dst_bytes = self.allocation.size() as usize;
                std::ptr::copy_nonoverlapping(src, dst, std::cmp::min(src_bytes, dst_bytes));
            } else {
                // This is expensive and should not be done in a hot loop
                let staging_buffer = Buffer::create_buffer(
                    device,
                    self.size,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    gpu_allocator::MemoryLocation::CpuToGpu,
                );

                let dst = staging_buffer.allocation.mapped_ptr().unwrap().as_ptr() as *mut u8;
                let dst_bytes = staging_buffer.allocation.size() as usize;
                std::ptr::copy_nonoverlapping(src, dst, std::cmp::min(src_bytes, dst_bytes));

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
                    .gpu_allocator
                    .lock()
                    .unwrap()
                    .free(staging_buffer.allocation)
                    .unwrap();
                device.handle.destroy_buffer(staging_buffer.buffer, None);
            }
        }
    }

    pub fn copy_to_buffer(&self, device: &Device, cb: vk::CommandBuffer, dst: &Buffer) {
        let buffer_copy_regions = vk::BufferCopy::builder()
            .size(self.size)
            .src_offset(0)
            .dst_offset(0)
            .build();

        unsafe {
            device
                .handle
                .cmd_copy_buffer(cb, self.buffer, dst.buffer, &[buffer_copy_regions]);
        }
    }

    pub fn copy_to_image(&self, device: &Device, cb: vk::CommandBuffer, image: &Image) {
        let buffer_copy_regions = vk::BufferImageCopy::builder()
            .image_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(image.desc.aspect_flags)
                    .layer_count(1)
                    .build(),
            )
            .image_extent(vk::Extent3D {
                width: image.width(),
                height: image.height(),
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

    pub fn set_debug_name(&mut self, device: &Device, name: &str) {
        self.debug_name = String::from(name);
        device.set_debug_name(
            vk::Handle::as_raw(self.buffer),
            vk::ObjectType::BUFFER,
            name,
        );
    }
}
