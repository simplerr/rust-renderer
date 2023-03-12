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
    pub render_func:
        Option<Box<dyn Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass, &GraphResources)>>,
    pub reads: Vec<TextureId>,
    pub writes: Vec<TextureWrite>,
    pub depth_attachment: Option<DepthAttachment>,
    pub presentation_pass: bool,
    pub read_textures_descriptor_set: Option<crate::DescriptorSet>,
    pub name: String,
    pub uniforms: HashMap<String, (String, UniformData)>,
    pub uniform_buffer: Option<BufferId>,
    pub uniforms_descriptor_set: Option<crate::DescriptorSet>,
}

impl RenderPass {
    pub fn new(
        name: String,
        pipeline_handle: PipelineId,
        presentation_pass: bool,
        depth_attachment: Option<DepthAttachment>,
        uniforms: HashMap<String, (String, UniformData)>,
        render_func: Option<
            Box<dyn Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass, &GraphResources)>,
        >,
    ) -> RenderPass {
        RenderPass {
            pipeline_handle,
            render_func,
            reads: vec![],
            writes: vec![],
            depth_attachment,
            presentation_pass,
            read_textures_descriptor_set: None,
            name,
            uniforms,
            uniform_buffer: None,
            uniforms_descriptor_set: None,
        }
    }

    pub fn try_create_read_texture_descriptor_set(
        &mut self,
        device: &Device,
        pipelines: &Vec<Pipeline>,
        textures: &Vec<GraphTexture>,
    ) {
        puffin::profile_function!();

        // If there are input textures then create the descriptor set used to read them
        if self.reads.len() > 0 && self.read_textures_descriptor_set.is_none() {
            let descriptor_set_input_textures = crate::DescriptorSet::new(
                &device,
                pipelines[self.pipeline_handle].descriptor_set_layouts
                    [crate::DESCRIPTOR_SET_INDEX_INPUT_TEXTURES as usize],
                pipelines[self.pipeline_handle]
                    .reflection
                    .get_set_mappings(crate::DESCRIPTOR_SET_INDEX_INPUT_TEXTURES),
            );

            for (idx, &read) in self.reads.iter().enumerate() {
                descriptor_set_input_textures.write_combined_image(
                    &device,
                    DescriptorIdentifier::Index(idx as u32),
                    &textures[read].texture,
                );
            }

            self.read_textures_descriptor_set
                .replace(descriptor_set_input_textures);
        }
    }

    pub fn try_create_uniform_buffer_descriptor_set(
        &mut self,
        device: &Device,
        pipelines: &Vec<Pipeline>,
        buffers: &Vec<crate::Buffer>,
    ) {
        puffin::profile_function!();

        if self.uniforms.len() > 0 && self.uniforms_descriptor_set.is_none() {
            // Todo: the usage of self.uniforms.values().next().unwrap() means
            // that only a single uniform buffer is supported

            // Todo: why unexpected size of 8 from size_of_val?

            // Create the descriptor set that uses the uniform buffer
            let uniform_name = &self.uniforms.values().next().unwrap().0;
            let binding = pipelines[self.pipeline_handle]
                .reflection
                .get_binding(&uniform_name);
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
                    &buffers[self.uniform_buffer.unwrap()],
                );
            }

            self.uniforms_descriptor_set.replace(descriptor_set);
        }
    }

    pub fn update_uniform_buffer_memory(
        &mut self,
        device: &Device,
        buffers: &mut Vec<crate::Buffer>,
    ) {
        puffin::profile_function!();

        if let Some(buffer_id) = self.uniform_buffer {
            buffers[buffer_id].update_memory(device, &self.uniforms.values().next().unwrap().1.data)
        }
    }

    pub fn prepare_render(
        &self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        color_attachments: &[Image],
        depth_attachment: Option<(Image, ViewType)>,
        extent: vk::Extent2D,
        pipelines: &Vec<Pipeline>,
    ) {
        let color_attachments = color_attachments
            .iter()
            .map(|image| {
                vk::RenderingAttachmentInfo::builder()
                    .image_view(image.image_view)
                    .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
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
                    .load_op(vk::AttachmentLoadOp::CLEAR)
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
                extent: extent,
            }];

            device
                .handle
                .cmd_set_viewport(command_buffer, 0, &viewports);
            device.handle.cmd_set_scissor(command_buffer, 0, &scissors);
        }
    }
}
