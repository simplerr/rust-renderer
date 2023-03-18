use ash::vk;
use ash::vk::{AccessFlags, ImageLayout, PipelineStageFlags};

use crate::Device;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageType {
    Tex1d = 0,
    Tex1dArray = 1,
    Tex2d = 2,
    Tex2dArray = 3,
    Tex3d = 4,
    Cube = 5,
    CubeArray = 6,
}

#[derive(Copy, Clone, Debug)]
pub struct ImageDesc {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub array_layers: u32,
    pub format: vk::Format,
    pub image_type: ImageType,
    pub aspect_flags: vk::ImageAspectFlags,
    pub usage: vk::ImageUsageFlags,
}

impl ImageDesc {
    pub fn new_2d(width: u32, height: u32, format: vk::Format) -> Self {
        ImageDesc {
            width,
            height,
            depth: 1,
            array_layers: 1,
            format,
            image_type: ImageType::Tex2d,
            aspect_flags: vk::ImageAspectFlags::COLOR,
            usage: vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            // Todo: better way to set common usage flags
        }
    }

    pub fn new_2d_array(width: u32, height: u32, array_layers: u32, format: vk::Format) -> Self {
        ImageDesc {
            width,
            height,
            depth: 1,
            array_layers,
            format,
            image_type: ImageType::Tex2dArray,
            aspect_flags: vk::ImageAspectFlags::COLOR,
            usage: vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            // Todo: better way to set common usage flags
        }
    }

    pub fn new_cubemap(width: u32, height: u32, format: vk::Format) -> Self {
        ImageDesc {
            width,
            height,
            depth: 1,
            array_layers: 6,
            format,
            image_type: ImageType::Cube,
            aspect_flags: vk::ImageAspectFlags::COLOR,
            usage: vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            // Todo: better way to set common usage flags
        }
    }

    pub fn aspect(mut self, aspect_flags: vk::ImageAspectFlags) -> Self {
        self.aspect_flags = aspect_flags;
        self
    }

    pub fn usage(mut self, usage_flags: vk::ImageUsageFlags) -> Self {
        self.usage = usage_flags;
        self
    }
}

// Todo: Hack
#[derive(Clone)]
pub struct Image {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub layer_views: Vec<vk::ImageView>,
    pub device_memory: vk::DeviceMemory,
    pub current_layout: vk::ImageLayout,
    pub desc: ImageDesc,
}

impl Image {
    pub fn new_from_desc(device: &Device, desc: ImageDesc) -> Image {
        puffin::profile_function!();

        unsafe {
            // Create image
            let initial_layout = vk::ImageLayout::UNDEFINED;
            let image_create_info = vk::ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: desc.format,
                extent: vk::Extent3D {
                    width: desc.width,
                    height: desc.height,
                    depth: 1,
                },
                mip_levels: 1,
                array_layers: desc.array_layers,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::OPTIMAL,
                usage: desc.usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                initial_layout,
                flags: if desc.image_type == ImageType::Cube
                    || desc.image_type == ImageType::CubeArray
                {
                    vk::ImageCreateFlags::CUBE_COMPATIBLE
                } else {
                    vk::ImageCreateFlags::empty()
                },
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

            let view_type = if desc.image_type == ImageType::Tex2d && desc.array_layers == 1 {
                vk::ImageViewType::TYPE_2D
            } else if desc.image_type == ImageType::Tex2dArray && desc.array_layers > 1 {
                vk::ImageViewType::TYPE_2D_ARRAY
            } else if desc.image_type == ImageType::Cube {
                vk::ImageViewType::CUBE
            } else {
                unimplemented!()
            };

            let image_view = Image::create_image_view(
                device,
                image,
                desc.format,
                desc.aspect_flags,
                view_type,
                0,
                desc.array_layers,
            );

            let mut layer_views = vec![];

            if desc.array_layers > 1
            /*&& desc.image_type == ImageType::Tex2dArray */
            {
                for layer in 0..desc.array_layers {
                    let view = Image::create_image_view(
                        device,
                        image,
                        desc.format,
                        desc.aspect_flags,
                        if desc.image_type == ImageType::Cube {
                            vk::ImageViewType::TYPE_2D
                        } else {
                            view_type
                        },
                        layer,
                        1,
                    );
                    layer_views.push(view);
                }
            }

            Image {
                image,
                image_view,
                layer_views,
                device_memory,
                current_layout: initial_layout,
                desc,
            }
        }
    }

    pub fn new_from_handle(device: &Device, image: vk::Image, desc: ImageDesc) -> Image {
        let view_type = if desc.image_type == ImageType::Tex2d && desc.array_layers == 1 {
            vk::ImageViewType::TYPE_2D
        } else if desc.image_type == ImageType::Tex2dArray && desc.array_layers > 1 {
            vk::ImageViewType::TYPE_2D_ARRAY
        } else if desc.image_type == ImageType::Cube {
            vk::ImageViewType::CUBE
        } else {
            unimplemented!()
        };

        let image_view = Image::create_image_view(
            device,
            image,
            desc.format,
            desc.aspect_flags,
            view_type,
            0,
            1,
        );

        Image {
            image,
            image_view,
            layer_views: vec![],
            device_memory: vk::DeviceMemory::null(),
            current_layout: vk::ImageLayout::UNDEFINED,
            desc,
        }
    }

    pub fn create_image_view(
        device: &Device,
        image: vk::Image,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
        view_type: vk::ImageViewType,
        base_array_layer: u32,
        layer_count: u32,
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
            view_type,
            format,
            components,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: aspect_flags,
                base_array_layer,
                level_count: 1,
                layer_count,
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
                aspect_mask: self.desc.aspect_flags,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .dst_subresource(vk::ImageSubresourceLayers {
                aspect_mask: dest.desc.aspect_flags,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .dst_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .extent(vk::Extent3D {
                width: self.desc.width,
                height: self.desc.height,
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
            ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL => (
                // Note: random flags, no idea if correct
                AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
                PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            ),
            _ => unimplemented!(),
        };

        let texture_barrier = vk::ImageMemoryBarrier {
            src_access_mask,
            dst_access_mask,
            new_layout,
            image: self.image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: self.desc.aspect_flags,
                level_count: 1,
                layer_count: self.desc.array_layers,
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

    pub fn layer_view(&self, layer: u32) -> vk::ImageView {
        assert!(layer < self.layer_views.len() as u32);
        self.layer_views[layer as usize]
    }

    pub fn width(&self) -> u32 {
        self.desc.width
    }

    pub fn height(&self) -> u32 {
        self.desc.height
    }

    pub fn format(&self) -> vk::Format {
        self.desc.format
    }

    pub fn is_depth_image_fmt(format: vk::Format) -> bool {
        format == vk::Format::D32_SFLOAT
            || format == vk::Format::D32_SFLOAT_S8_UINT
            || format == vk::Format::D16_UNORM_S8_UINT
            || format == vk::Format::D16_UNORM
            || format == vk::Format::D24_UNORM_S8_UINT
    }
}
