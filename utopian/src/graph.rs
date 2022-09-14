use ash::vk;

use crate::descriptor_set::DescriptorIdentifier;
use crate::device::*;
use crate::image::*;
use crate::Pipeline;
use crate::RenderPass;
use crate::Renderer;
use crate::Texture;

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum GraphResourceId {
    ColoredRectTexture,
    PbrOutputTexture,
}

pub type TextureId = usize;

pub struct GraphResource {
    pub texture: Texture,
    pub prev_access: vk_sync::AccessType,
}

pub struct Graph {
    pub passes: Vec<RenderPass>,
    pub resources: Vec<GraphResource>,
}

pub struct PassBuilder<'a> {
    pub graph: &'a mut Graph,
    pub name: String,
    pub pipeline: Pipeline,
    pub reads: Vec<TextureId>,
    pub writes: Vec<TextureId>,
    pub render_func: Option<Box<dyn Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass)>>,
    pub depth_attachment: Option<Image>,
    pub presentation_pass: bool,
}

impl<'a> PassBuilder<'a> {
    pub fn read(mut self, resource_id: TextureId) -> Self {
        self.reads.push(resource_id);
        self
    }

    pub fn write(mut self, resource_id: TextureId) -> Self {
        self.writes.push(resource_id);
        self
    }

    pub fn render(
        mut self,
        render_func: impl Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass) + 'static,
    ) -> Self {
        self.render_func.replace(Box::new(render_func));
        self
    }

    pub fn presentation_pass(mut self, is_presentation_pass: bool) -> Self {
        self.presentation_pass = is_presentation_pass;
        self
    }

    pub fn depth_attachment(mut self, depth_attachment: Image) -> Self {
        self.depth_attachment = Some(depth_attachment);
        self
    }

    pub fn build(self, device: &Device) {
        let mut pass = crate::RenderPass::new(
            self.pipeline,
            self.presentation_pass,
            self.depth_attachment,
            self.render_func,
        );

        for read in &self.reads {
            pass.reads.push(*read);
        }

        for write in &self.writes {
            pass.writes.push(*write);
        }

        // If there are input textures then create the descriptor set used to read them
        if self.reads.len() > 0 {
            let descriptor_set_input_textures = crate::DescriptorSet::new(
                &device,
                pass.pipeline.descriptor_set_layouts
                    [crate::DESCRIPTOR_SET_INDEX_INPUT_TEXTURES as usize],
                pass.pipeline
                    .reflection
                    .get_set_mappings(crate::DESCRIPTOR_SET_INDEX_INPUT_TEXTURES),
            );

            for (idx, &read) in self.reads.iter().enumerate() {
                descriptor_set_input_textures.write_combined_image(
                    &device,
                    DescriptorIdentifier::Index(idx as u32),
                    &self.graph.resources[read].texture,
                );
            }

            pass.read_textures_descriptor_set
                .replace(descriptor_set_input_textures);
        }

        self.graph.passes.push(pass);
    }
}

impl Graph {
    pub fn add_pass(&mut self, name: String, pipeline: Pipeline) -> PassBuilder {
        PassBuilder {
            graph: self,
            name,
            pipeline,
            reads: vec![],
            writes: vec![],
            render_func: None,
            depth_attachment: None,
            presentation_pass: false,
        }
    }

    pub fn create_texture(&mut self, device: &crate::Device, width: u32, height: u32) -> TextureId {
        let new_texture = crate::Texture::create(&device, None, width, height);
        self.resources.push(GraphResource {
            texture: new_texture,
            prev_access: vk_sync::AccessType::Nothing,
        });

        self.resources.len() - 1
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
                let next_access = crate::synch::image_pipeline_barrier(
                    &device,
                    command_buffer,
                    &self.resources[*read].texture.image,
                    self.resources[*read].prev_access,
                    vk_sync::AccessType::AnyShaderReadSampledImageOrUniformTexelBuffer,
                );

                self.resources.get_mut(*read).unwrap().prev_access = next_access;
            }

            for write in &pass.writes {
                let next_access = crate::synch::image_pipeline_barrier(
                    &device,
                    command_buffer,
                    &self.resources[*write].texture.image,
                    self.resources[*write].prev_access,
                    vk_sync::AccessType::ColorAttachmentWrite,
                );

                self.resources.get_mut(*write).unwrap().prev_access = next_access;
            }

            if pass.presentation_pass {
                crate::synch::image_pipeline_barrier(
                    &device,
                    command_buffer,
                    &present_image[0],
                    vk_sync::AccessType::Present,
                    vk_sync::AccessType::ColorAttachmentWrite,
                );
            }

            let write_attachments: Vec<Image> = pass
                .writes
                .iter()
                .map(|write| self.resources[*write].texture.image)
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
                if !pass.presentation_pass {
                    vk::Extent2D {
                        width: self.resources[pass.writes[0]].texture.image.width, // Todo
                        height: self.resources[pass.writes[0]].texture.image.height, // Todo
                    }
                } else {
                    vk::Extent2D {
                        width: present_image[0].width,   // Todo
                        height: present_image[0].height, // Todo
                    }
                },
            );

            if let Some(read_textures_descriptor_set) = &pass.read_textures_descriptor_set {
                unsafe {
                    device.handle.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pass.pipeline.pipeline_layout,
                        crate::DESCRIPTOR_SET_INDEX_INPUT_TEXTURES,
                        &[read_textures_descriptor_set.handle],
                        &[],
                    )
                };
            }

            if let Some(render_func) = &pass.render_func {
                render_func(device, command_buffer, renderer, pass);
            }

            unsafe { device.handle.cmd_end_rendering(command_buffer) };
        }
    }
}
