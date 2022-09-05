use ash::vk;

use crate::device::*;
use crate::image::*;
use crate::pipeline::*;

pub struct RenderPass {
    pub pipeline: Pipeline,
}

impl RenderPass {
    pub fn new(
        device: &ash::Device,
        pipeline_desc: PipelineDesc,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
        color_attachments: &[&Image],
    ) -> RenderPass {
        let pipeline = Pipeline::new(
            &device,
            pipeline_desc,
            color_attachments
                .iter()
                .map(|image| image.format)
                .collect::<Vec<_>>()
                .as_slice(),
            vk::Format::D32_SFLOAT, // Todo
            bindless_descriptor_set_layout,
        );

        RenderPass { pipeline }
    }

    pub fn prepare_render(
        &self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        color_attachments: &[&Image],
        depth_attachment: Option<&Image>,
        extent: vk::Extent2D,
    ) {
        vk_sync::cmd::pipeline_barrier(&device.handle,
            command_buffer,
            None,
            &[],
            &[vk_sync::ImageBarrier {
                previous_accesses: &[vk_sync::AccessType::Nothing],
                next_accesses: &[vk_sync::AccessType::ColorAttachmentWrite],
                previous_layout: vk_sync::ImageLayout::Optimal,
                next_layout: vk_sync::ImageLayout::Optimal,
                discard_contents: false,
                src_queue_family_index: 0,
                dst_queue_family_index: 0,
                image: color_attachments[0].image, // Todo transition all images
                range: vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .layer_count(1)
                .level_count(1)
                .build(),
            }]);

        let rendering_info = vk::RenderingInfo::builder()
            .layer_count(1)
            .color_attachments(&[vk::RenderingAttachmentInfo::builder()
                .image_view(
                    color_attachments[0].image_view, // Todo
                )
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.5, 0.5, 0.5, 0.0],
                    },
                })
                .build()])
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
