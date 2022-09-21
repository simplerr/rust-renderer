use ash::vk;

use crate::device::*;
use crate::graph::*;
use crate::image::*;
use crate::pipeline::*;
use crate::Renderer;

pub struct RenderPass {
    pub pipeline: Pipeline,
    pub render_func: Option<Box<dyn Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass)>>,
    pub reads: Vec<TextureId>,
    pub writes: Vec<TextureId>,
    pub depth_attachment: Option<Image>,
    pub presentation_pass: bool,
    pub read_textures_descriptor_set: Option<crate::DescriptorSet>,
    pub name: String,
}

impl RenderPass {
    pub fn new(
        name: String,
        pipeline: Pipeline,
        presentation_pass: bool,
        depth_attachment: Option<Image>,
        render_func: Option<Box<dyn Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass)>>,
    ) -> RenderPass {
        RenderPass {
            pipeline,
            render_func,
            reads: vec![],
            writes: vec![],
            depth_attachment,
            presentation_pass,
            read_textures_descriptor_set: None,
            name,
        }
    }

    pub fn prepare_render(
        &self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        color_attachments: &[Image],
        depth_attachment: Option<Image>,
        extent: vk::Extent2D,
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
                            float32: [0.5, 0.5, 0.5, 0.0],
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
                    .image_view(depth_attachment.image_view)
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
                self.pipeline.handle,
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
