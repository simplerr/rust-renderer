use ash::vk;

use crate::device::*;
use crate::image::*;
use crate::RenderPass;
use crate::Renderer;

use std::collections::HashMap;

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum GraphResourceId {
    ColoredRectTexture,
    PbrOutputTexture,
}

pub struct GraphResource {
    pub image: Image,
    pub prev_acces: vk_sync::AccessType,
}

pub struct Graph {
    pub passes: Vec<RenderPass>,
    pub resources: HashMap<GraphResourceId, GraphResource>,
}

impl Graph {
    pub fn add_pass(
        &mut self,
        reads: &[(GraphResourceId, Image)],
        writes: &[(GraphResourceId, Image)],
        mut pass: RenderPass,
    ) {
        for read in reads {
            self.resources.insert(
                read.0,
                GraphResource {
                    image: read.1,
                    prev_acces: vk_sync::AccessType::Nothing,
                },
            );
            pass.reads.push(read.0);
        }

        for write in writes {
            self.resources.insert(
                write.0,
                GraphResource {
                    image: write.1,
                    prev_acces: vk_sync::AccessType::Nothing,
                },
            );
            pass.writes.push(write.0);
        }

        self.passes.push(pass);
    }

    pub fn render(
        &mut self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        renderer: &Renderer,
        present_image: &[Image], // Todo: pass single value
    ) {
        for pass in &self.passes {
            // Transition pass resources
            // Todo: probably can combine reads and writes to one vector
            for read in &pass.reads {
                let prev_access = self.resources[read].prev_acces;
                let next_access =
                    vk_sync::AccessType::AnyShaderReadSampledImageOrUniformTexelBuffer; // Todo: shall be argument to read() builder

                vk_sync::cmd::pipeline_barrier(
                    &device.handle,
                    command_buffer,
                    None,
                    &[],
                    &[vk_sync::ImageBarrier {
                        previous_accesses: &[prev_access],
                        next_accesses: &[next_access],
                        previous_layout: vk_sync::ImageLayout::Optimal,
                        next_layout: vk_sync::ImageLayout::Optimal,
                        discard_contents: false,
                        src_queue_family_index: 0,
                        dst_queue_family_index: 0,
                        image: self.resources[read].image.image, // Todo transition all images
                        range: vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .level_count(1)
                            .build(),
                    }],
                );

                self.resources.get_mut(read).unwrap().prev_acces = next_access;
            }

            for write in &pass.writes {
                let mut prev_access = self.resources[write].prev_acces;
                let next_access = vk_sync::AccessType::ColorAttachmentWrite;

                if pass.presentation_pass {
                    prev_access = vk_sync::AccessType::Present;
                }

                vk_sync::cmd::pipeline_barrier(
                    &device.handle,
                    command_buffer,
                    None,
                    &[],
                    &[vk_sync::ImageBarrier {
                        previous_accesses: &[prev_access],
                        next_accesses: &[vk_sync::AccessType::ColorAttachmentWrite],
                        previous_layout: vk_sync::ImageLayout::Optimal,
                        next_layout: vk_sync::ImageLayout::Optimal,
                        discard_contents: false,
                        src_queue_family_index: 0,
                        dst_queue_family_index: 0,
                        //image: write.image, // Todo transition all images
                        // Todo
                        image: if !pass.presentation_pass {
                            self.resources[write].image.image
                        } else {
                            present_image[0].image
                        },
                        range: vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .level_count(1)
                            .build(),
                    }],
                );

                self.resources.get_mut(write).unwrap().prev_acces = next_access;
            }

            let write_attachments: Vec<Image> = pass
                .writes
                .iter()
                .map(|write| self.resources[write].image)
                .collect();

            pass.prepare_render(
                device,
                command_buffer,
                if !pass.presentation_pass {
                    write_attachments.as_slice()
                } else {
                    present_image
                },
                pass.depth_attachment,
                vk::Extent2D {
                    width: self.resources[&pass.writes[0]].image.width, // Todo
                    height: self.resources[&pass.writes[0]].image.height, // Todo
                },
            );

            if let Some(render_func) = &pass.render_func {
                render_func(device, command_buffer, renderer, pass);
            }

            unsafe { device.handle.cmd_end_rendering(command_buffer) };
        }
    }
}
