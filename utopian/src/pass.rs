use std::collections::HashMap;

use ash::vk;

use crate::descriptor_set::DescriptorIdentifier;
use crate::device::*;
use crate::graph::*;
use crate::image::*;
use crate::pipeline::*;
use crate::Renderer;

pub struct RenderPass {
    pub pipeline_handle: PipelineId,
    #[allow(clippy::type_complexity)]
    pub render_func:
        Option<Box<dyn Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass, &GraphResources)>>,
    pub reads: Vec<Resource>,
    pub writes: Vec<Attachment>,
    pub depth_attachment: Option<DepthAttachment>,
    pub presentation_pass: bool,
    pub read_resources_descriptor_set: Option<crate::DescriptorSet>,
    pub name: String,
    pub uniforms: HashMap<String, (String, UniformData)>,
    pub uniform_buffer: Option<BufferId>,
    pub uniforms_descriptor_set: Option<crate::DescriptorSet>,
    pub copy_command: Option<TextureCopy>,
    pub extra_barriers: Option<Vec<(BufferId, vk_sync::AccessType)>>,
}

impl RenderPass {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        pipeline_handle: PipelineId,
        presentation_pass: bool,
        depth_attachment: Option<DepthAttachment>,
        uniforms: HashMap<String, (String, UniformData)>,
        #[allow(clippy::type_complexity)] render_func: Option<
            Box<dyn Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass, &GraphResources)>,
        >,
        copy_command: Option<TextureCopy>,
        extra_barriers: Option<Vec<(BufferId, vk_sync::AccessType)>>,
    ) -> RenderPass {
        RenderPass {
            pipeline_handle,
            render_func,
            reads: vec![],
            writes: vec![],
            depth_attachment,
            presentation_pass,
            read_resources_descriptor_set: None,
            name,
            uniforms,
            uniform_buffer: None,
            uniforms_descriptor_set: None,
            copy_command,
            extra_barriers,
        }
    }

    pub fn try_create_read_resources_descriptor_set(
        &mut self,
        device: &Device,
        pipelines: &[Pipeline],
        textures: &[GraphTexture],
        buffers: &[GraphBuffer],
        tlas: vk::AccelerationStructureKHR,
    ) {
        puffin::profile_function!();

        // If there are input textures then create the descriptor set used to read them
        if !self.reads.is_empty() && self.read_resources_descriptor_set.is_none() {
            let descriptor_set_read_resources = crate::DescriptorSet::new(
                device,
                pipelines[self.pipeline_handle].descriptor_set_layouts
                    [crate::DESCRIPTOR_SET_INDEX_INPUT_TEXTURES as usize],
                pipelines[self.pipeline_handle]
                    .reflection
                    .get_set_mappings(crate::DESCRIPTOR_SET_INDEX_INPUT_TEXTURES),
            );

            for (idx, &read) in self.reads.iter().enumerate() {
                match read {
                    Resource::Texture(read) => {
                        if read.input_type == TextureResourceType::CombinedImageSampler {
                            descriptor_set_read_resources.write_combined_image(
                                device,
                                DescriptorIdentifier::Index(idx as u32),
                                &textures[read.texture].texture,
                            );
                        } else if read.input_type == TextureResourceType::StorageImage {
                            descriptor_set_read_resources.write_storage_image(
                                device,
                                DescriptorIdentifier::Index(idx as u32),
                                &textures[read.texture].texture.image,
                            );
                        }
                    }
                    Resource::Buffer(read) => {
                        descriptor_set_read_resources.write_storage_buffer(
                            device,
                            DescriptorIdentifier::Index(idx as u32),
                            &buffers[read.buffer].buffer,
                        );
                    }
                    // The acceleration structure is specially treated for now since it is
                    // an external resource not owned by the graph
                    Resource::Tlas(_) => {
                        assert!(tlas != vk::AccelerationStructureKHR::null());
                        descriptor_set_read_resources.write_acceleration_structure(
                            device,
                            DescriptorIdentifier::Index(idx as u32),
                            tlas,
                        );
                    }
                }
            }

            self.read_resources_descriptor_set
                .replace(descriptor_set_read_resources);
        }
    }

    pub fn try_create_uniform_buffer_descriptor_set(
        &mut self,
        device: &Device,
        pipelines: &[Pipeline],
        buffers: &[GraphBuffer],
    ) {
        puffin::profile_function!();

        if !self.uniforms.is_empty() && self.uniforms_descriptor_set.is_none() {
            // Todo: the usage of self.uniforms.values().next().unwrap() means
            // that only a single uniform buffer is supported

            // Todo: why unexpected size of 8 from size_of_val?

            // Create the descriptor set that uses the uniform buffer
            let uniform_name = &self.uniforms.values().next().unwrap().0;
            let binding = pipelines[self.pipeline_handle]
                .reflection
                .get_binding(uniform_name);
            let descriptor_set = crate::DescriptorSet::new(
                device,
                pipelines[self.pipeline_handle].descriptor_set_layouts[binding.set as usize],
                pipelines[self.pipeline_handle]
                    .reflection
                    .get_set_mappings(binding.set),
            );
            {
                descriptor_set.write_uniform_buffer(
                    device,
                    uniform_name.to_string(),
                    &buffers[self.uniform_buffer.unwrap()].buffer,
                );
            }

            self.uniforms_descriptor_set.replace(descriptor_set);
        }
    }

    pub fn update_uniform_buffer_memory(&mut self, device: &Device, buffers: &mut [GraphBuffer]) {
        puffin::profile_function!();

        if let Some(buffer_id) = self.uniform_buffer {
            buffers[buffer_id]
                .buffer
                .update_memory(device, &self.uniforms.values().next().unwrap().1.data)
        }
    }

    pub fn prepare_render(
        &self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        color_attachments: &[(Image, ViewType, vk::AttachmentLoadOp)],
        depth_attachment: Option<(Image, ViewType, vk::AttachmentLoadOp)>,
        extent: vk::Extent2D,
        pipelines: &[Pipeline],
    ) {
        let bind_point = match pipelines[self.pipeline_handle].pipeline_type {
            PipelineType::Graphics => vk::PipelineBindPoint::GRAPHICS,
            PipelineType::Compute => vk::PipelineBindPoint::COMPUTE,
            PipelineType::Raytracing => vk::PipelineBindPoint::RAY_TRACING_KHR,
        };

        if bind_point != vk::PipelineBindPoint::GRAPHICS {
            unsafe {
                device.handle.cmd_bind_pipeline(
                    command_buffer,
                    bind_point,
                    pipelines[self.pipeline_handle].handle,
                );
            }

            return;
        }

        let color_attachments = color_attachments
            .iter()
            .map(|image| {
                vk::RenderingAttachmentInfo::builder()
                    .image_view(match image.1 {
                        ViewType::Full() => image.0.image_view,
                        ViewType::Layer(layer) => image.0.layer_view(layer),
                    })
                    .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .load_op(image.2)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [1.0, 1.0, 1.0, 0.0],
                        },
                    })
                    .build()
            })
            .collect::<Vec<_>>();

        let rendering_info = vk::RenderingInfo::builder()
            .layer_count(1)
            .color_attachments(&color_attachments)
            .depth_attachment(&if let Some(depth_attachment) = depth_attachment {
                vk::RenderingAttachmentInfo::builder()
                    .image_view(match depth_attachment.1 {
                        ViewType::Full() => depth_attachment.0.image_view,
                        ViewType::Layer(layer) => depth_attachment.0.layer_view(layer),
                    })
                    .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .load_op(depth_attachment.2)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.0,
                            stencil: 0,
                        },
                    })
                    .build()
            } else {
                vk::RenderingAttachmentInfo::default()
            })
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            })
            .build();

        unsafe {
            device
                .handle
                .cmd_begin_rendering(command_buffer, &rendering_info);

            device.handle.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipelines[self.pipeline_handle].handle,
            );

            let viewports = [vk::Viewport {
                x: 0.0,
                y: extent.height as f32,
                width: extent.width as f32,
                height: -(extent.height as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            }];

            let scissors = [vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            }];

            device
                .handle
                .cmd_set_viewport(command_buffer, 0, &viewports);
            device.handle.cmd_set_scissor(command_buffer, 0, &scissors);
        }
    }
}
