use ash::vk;
use ash::vk::{AccessFlags, ImageLayout, PipelineStageFlags};

use crate::Device;

// Todo: Hack
#[derive(Copy, Clone)]
pub struct Image {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub device_memory: vk::DeviceMemory,
    pub current_layout: vk::ImageLayout,
    pub format: vk::Format,
    pub width: u32,
    pub height: u32,
}

impl Image {
    pub fn new(
        device: &Device,
        width: u32,
        height: u32,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        aspect_flags: vk::ImageAspectFlags,
    ) -> Image {
        puffin::profile_function!();

        unsafe {
            // Create image
            let initial_layout = vk::ImageLayout::UNDEFINED;
            let image_create_info = vk::ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format,
                extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::OPTIMAL,
                usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                initial_layout,
                ..Default::default()
            };
            let image = device
                .handle
                .create_image(&image_create_info, None)
                .expect("Unable to create image");

            // Allocate and bind device memory
            let image_memory_req = device.handle.get_image_memory_requirements(image);
            let image_memory_index = device
                .find_memory_type_index(&image_memory_req, vk::MemoryPropertyFlags::DEVICE_LOCAL)
                .expect("Unable to find suitable memory index for image");
            let image_allocate_info = vk::MemoryAllocateInfo {
                allocation_size: image_memory_req.size,
                memory_type_index: image_memory_index,
                ..Default::default()
            };
            let device_memory = device
                .handle
                .allocate_memory(&image_allocate_info, None)
                .expect("Unable to allocate image device memory");

            device
                .handle
                .bind_image_memory(image, device_memory, 0)
                .expect("Unable to bind device memory to image");

            let image_view = Image::create_image_view(device, image, format, aspect_flags);

            Image {
                image,
                image_view,
                device_memory,
                current_layout: initial_layout,
                format,
                width,
                height,
            }
        }
    }

    pub fn new_from_handle(
        device: &Device,
        image: vk::Image,
        width: u32,
        height: u32,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
    ) -> Image {
        let image_view = Image::create_image_view(device, image, format, aspect_flags);

        Image {
            image,
            image_view,
            device_memory: vk::DeviceMemory::null(),
            current_layout: vk::ImageLayout::UNDEFINED,
            format,
            width,
            height,
        }
    }

    pub fn create_image_view(
        device: &Device,
        image: vk::Image,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
    ) -> vk::ImageView {
        // Create image view
        let components = match aspect_flags {
            vk::ImageAspectFlags::COLOR => vk::ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            },
            vk::ImageAspectFlags::DEPTH => vk::ComponentMapping::default(),
            _ => unimplemented!(),
        };

        let image_view_info = vk::ImageViewCreateInfo {
            view_type: vk::ImageViewType::TYPE_2D,
            format,
            components,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: aspect_flags,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            image,
            ..Default::default()
        };

        unsafe {
            device
                .handle
                .create_image_view(&image_view_info, None)
                .unwrap()
        }
    }

    pub fn copy(&self, device: &Device, cb: vk::CommandBuffer, dest: &Image) {
        let copy_region = vk::ImageCopy::builder()
            .src_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .dst_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .dst_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .extent(vk::Extent3D {
                width: self.width,
                height: self.height,
                depth: 1,
            })
            .build();

        unsafe {
            device.handle.cmd_copy_image(
                cb,
                self.image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                dest.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[copy_region],
            )
        };
    }

    pub fn transition_layout(
        &self,
        device: &Device,
        cb: vk::CommandBuffer,
        new_layout: vk::ImageLayout,
    ) {
        let (src_access_mask, src_stage_mask) = match self.current_layout {
            ImageLayout::UNDEFINED => (AccessFlags::HOST_WRITE, PipelineStageFlags::HOST),
            ImageLayout::PREINITIALIZED => (AccessFlags::HOST_WRITE, PipelineStageFlags::HOST),
            ImageLayout::TRANSFER_DST_OPTIMAL => {
                (AccessFlags::TRANSFER_WRITE, PipelineStageFlags::TRANSFER)
            }
            ImageLayout::TRANSFER_SRC_OPTIMAL => {
                (AccessFlags::TRANSFER_READ, PipelineStageFlags::TRANSFER)
            }
            ImageLayout::GENERAL => (AccessFlags::HOST_WRITE, PipelineStageFlags::HOST),
            ImageLayout::SHADER_READ_ONLY_OPTIMAL => {
                (AccessFlags::HOST_WRITE, PipelineStageFlags::HOST)
            }
            ImageLayout::COLOR_ATTACHMENT_OPTIMAL => (
                AccessFlags::COLOR_ATTACHMENT_WRITE,
                PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ),
            _ => unimplemented!(),
        };

        let (dst_access_mask, dst_stage_mask) = match new_layout {
            ImageLayout::TRANSFER_SRC_OPTIMAL => {
                (AccessFlags::TRANSFER_READ, PipelineStageFlags::TRANSFER)
            }
            ImageLayout::TRANSFER_DST_OPTIMAL => {
                (AccessFlags::TRANSFER_WRITE, PipelineStageFlags::TRANSFER)
            }
            ImageLayout::SHADER_READ_ONLY_OPTIMAL => (
                AccessFlags::SHADER_READ,
                PipelineStageFlags::FRAGMENT_SHADER,
            ),
            ImageLayout::COLOR_ATTACHMENT_OPTIMAL => (
                AccessFlags::COLOR_ATTACHMENT_WRITE,
                PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ),
            ImageLayout::GENERAL => (
                AccessFlags::SHADER_READ,
                PipelineStageFlags::FRAGMENT_SHADER,
            ),
            ImageLayout::PRESENT_SRC_KHR => (
                // Note: random flags, no idea if correct
                AccessFlags::COLOR_ATTACHMENT_READ,
                PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ),
            _ => unimplemented!(),
        };

        let texture_barrier = vk::ImageMemoryBarrier {
            src_access_mask,
            dst_access_mask,
            new_layout,
            image: self.image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        unsafe {
            device.handle.cmd_pipeline_barrier(
                cb,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[texture_barrier],
            );
        }
    }
}
