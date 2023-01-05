use std::collections::HashMap;

use ash::vk;

use crate::device::*;
use crate::image::*;
use crate::Buffer;
use crate::Pipeline;
use crate::PipelineDesc;
use crate::RenderPass;
use crate::Renderer;
use crate::Texture;

/// Virtual resource handles.
pub type TextureId = usize;
pub type BufferId = usize;
pub type PipelineId = usize;

pub struct GraphTexture {
    pub texture: Texture,
    pub prev_access: vk_sync::AccessType,
}

pub struct GraphResources {
    pub buffers: Vec<Buffer>,
    pub textures: Vec<GraphTexture>,
    pub pipelines: Vec<Pipeline>,
}
pub enum DepthAttachment {
    GraphTexture(TextureWrite),
    External(Image),
}

#[derive(Copy, Clone)]
pub enum ViewType {
    Full(),
    Layer(u32),
}

#[derive(Copy, Clone)]
pub struct TextureWrite {
    pub texture: TextureId,
    pub view: ViewType,
}

pub struct Graph {
    pub passes: Vec<RenderPass>,
    pub resources: GraphResources,
    pub descriptor_set_camera: crate::DescriptorSet,
    pub pipeline_descs: Vec<PipelineDesc>,
}

pub const MAX_UNIFORMS_SIZE: usize = 2048;

// Note: the way that the uniform data array is hardcoded in size might be a problem.
// The goal was to have a clear method for each pass to own its own data without adding complexity.
// A good improvement would be to use Dynamic Descriptor sets so that all data from the different
// passes is placed in the same buffer but with different offsets.
#[derive(Copy, Clone)]
pub struct UniformData {
    pub data: [u8; MAX_UNIFORMS_SIZE],
    pub size: u64,
}

pub struct PassBuilder {
    pub name: String,
    pub pipeline_handle: PipelineId,
    pub reads: Vec<TextureId>,
    pub writes: Vec<TextureWrite>,
    pub render_func:
        Option<Box<dyn Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass, &GraphResources)>>,
    pub depth_attachment: Option<DepthAttachment>,
    pub presentation_pass: bool,
    pub uniforms: HashMap<String, UniformData>,
}

impl GraphResources {
    fn new() -> GraphResources {
        GraphResources {
            buffers: vec![],
            textures: vec![],
            pipelines: vec![],
        }
    }

    pub fn buffer<'a>(&'a self, id: BufferId) -> &'a Buffer {
        &self.buffers[id]
    }

    pub fn texture<'a>(&'a self, id: TextureId) -> &'a GraphTexture {
        &self.textures[id]
    }

    pub fn pipeline<'a>(&'a self, id: PipelineId) -> &'a Pipeline {
        &self.pipelines[id]
    }
}

impl PassBuilder {
    pub fn read(mut self, resource_id: TextureId) -> Self {
        self.reads.push(resource_id);
        self
    }

    pub fn write(mut self, resource_id: TextureId) -> Self {
        self.writes.push(TextureWrite {
            texture: resource_id,
            view: ViewType::Full(),
        });
        self
    }

    pub fn render(
        mut self,
        render_func: impl Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass, &GraphResources)
            + 'static,
    ) -> Self {
        self.render_func.replace(Box::new(render_func));
        self
    }

    pub fn presentation_pass(mut self, is_presentation_pass: bool) -> Self {
        self.presentation_pass = is_presentation_pass;
        self
    }

    pub fn depth_attachment(mut self, depth_attachment: TextureId) -> Self {
        self.depth_attachment = Some(DepthAttachment::GraphTexture(TextureWrite {
            texture: depth_attachment,
            view: ViewType::Full(),
        }));
        self
    }

    pub fn depth_attachment_layer(mut self, depth_attachment: TextureId, layer: u32) -> Self {
        self.depth_attachment = Some(DepthAttachment::GraphTexture(TextureWrite {
            texture: depth_attachment,
            view: ViewType::Layer(layer),
        }));
        self
    }

    pub fn external_depth_attachment(mut self, depth_attachment: Image) -> Self {
        self.depth_attachment = Some(DepthAttachment::External(depth_attachment));
        self
    }

    pub fn uniforms<T: Copy + std::fmt::Debug>(mut self, name: &str, data: &T) -> Self {
        puffin::profile_function!();

        // Note: Todo: this can be improved
        unsafe {
            let ptr = data as *const T;
            let size = core::mem::size_of::<T>();
            let data_u8 = std::slice::from_raw_parts(ptr as *const u8, std::mem::size_of::<T>());

            assert!(data_u8.len() < MAX_UNIFORMS_SIZE);

            if let Some(entry) = self.uniforms.get_mut(name) {
                entry.data[..data_u8.len()].copy_from_slice(data_u8);
                entry.size = size as u64;
            } else {
                let mut new_entry = UniformData {
                    data: [0; MAX_UNIFORMS_SIZE],
                    size: size as u64,
                };
                new_entry.data[..data_u8.len()].copy_from_slice(data_u8);
                self.uniforms.insert(name.to_string(), new_entry);
            }
        }
        self
    }

    pub fn build(self, device: &Device, graph: &mut Graph) {
        let mut pass = crate::RenderPass::new(
            self.name,
            self.pipeline_handle,
            self.presentation_pass,
            self.depth_attachment,
            self.uniforms.clone(), // Note: is this clone OK?
            self.render_func,
        );

        for read in &self.reads {
            pass.reads.push(*read);
        }

        for write in &self.writes {
            pass.writes.push(*write);
        }

        if self.uniforms.len() != 0 {
            pass.uniform_buffer.replace(graph.create_buffer(
                &self.uniforms.keys().next().unwrap(),
                device,
                self.uniforms.values().next().unwrap().size as u64,
            ));
        }

        graph.passes.push(pass);
    }
}

impl Graph {
    pub fn new(device: &Device, camera_uniform_buffer: &Buffer) -> Self {
        Graph {
            passes: vec![],
            resources: GraphResources::new(),
            descriptor_set_camera: Self::create_camera_descriptor_set(
                device,
                camera_uniform_buffer,
            ),
            pipeline_descs: vec![],
        }
    }

    pub fn clear(&mut self, device: &crate::Device) {
        puffin::profile_function!();

        for pass in &self.passes {
            if let Some(descriptor_set) = &pass.uniforms_descriptor_set {
                unsafe {
                    device
                        .handle
                        .destroy_descriptor_pool(descriptor_set.pool, None)
                };
            }
            if let Some(descriptor_set) = &pass.read_textures_descriptor_set {
                unsafe {
                    device
                        .handle
                        .destroy_descriptor_pool(descriptor_set.pool, None)
                };
            }
        }

        self.passes.clear();
        self.pipeline_descs.clear();
    }

    pub fn create_camera_descriptor_set(
        device: &Device,
        camera_uniform_buffer: &Buffer,
    ) -> crate::DescriptorSet {
        let descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::ALL)
            .build();

        let descriptor_sets_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&[descriptor_set_layout_binding])
            .build();

        let descriptor_set_layout = unsafe {
            device
                .handle
                .create_descriptor_set_layout(&descriptor_sets_layout_info, None)
                .expect("Error creating descriptor set layout")
        };

        let mut binding_map: crate::shader::BindingMap = std::collections::BTreeMap::new();
        binding_map.insert(
            "view".to_string(),
            crate::shader::Binding {
                set: crate::DESCRIPTOR_SET_INDEX_VIEW,
                binding: 0,
                info: rspirv_reflect::DescriptorInfo {
                    ty: rspirv_reflect::DescriptorType::UNIFORM_BUFFER,
                    binding_count: rspirv_reflect::BindingCount::One,
                    name: "view".to_string(),
                },
            },
        );

        let descriptor_set_camera =
            crate::DescriptorSet::new(&device, descriptor_set_layout, binding_map);

        descriptor_set_camera.write_uniform_buffer(
            &device,
            "view".to_string(),
            &camera_uniform_buffer,
        );

        descriptor_set_camera
    }

    pub fn add_pass(&mut self, name: String, pipeline_handle: PipelineId) -> PassBuilder {
        PassBuilder {
            name,
            pipeline_handle,
            reads: vec![],
            writes: vec![],
            render_func: None,
            depth_attachment: None,
            presentation_pass: false,
            uniforms: HashMap::new(),
        }
    }

    pub fn create_texture(
        &mut self,
        debug_name: &str,
        device: &crate::Device,
        image_desc: ImageDesc,
    ) -> TextureId {
        puffin::profile_function!();

        // Todo: Cannot rely on debug_name being unique
        // Todo: shall use a Hash to include extent and format of the texture
        self.resources
            .textures
            .iter()
            .position(|iter| iter.texture.debug_name == debug_name)
            .unwrap_or_else(|| {
                let mut texture = crate::Texture::create(&device, None, image_desc);
                texture.set_debug_name(device, debug_name);

                self.resources.textures.push(GraphTexture {
                    texture,
                    prev_access: vk_sync::AccessType::Nothing,
                });

                self.resources.textures.len() - 1
            })
    }

    pub fn create_buffer(
        &mut self,
        debug_name: &str,
        device: &crate::Device,
        size: u64,
    ) -> BufferId {
        // Todo: Cannot rely on debug_name being unique

        self.resources
            .buffers
            .iter()
            .position(|iter| iter.debug_name == debug_name)
            .unwrap_or_else(|| {
                let mut buffer = Buffer::new::<u8>(
                    device,
                    None,
                    size,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                    gpu_allocator::MemoryLocation::CpuToGpu,
                );

                buffer.set_debug_name(device, debug_name);

                self.resources.buffers.push(buffer);

                self.resources.buffers.len() - 1
            })
    }

    pub fn create_pipeline(&mut self, pipeline_desc: PipelineDesc) -> PipelineId {
        // Todo: need to check if it already exists
        self.pipeline_descs.push(pipeline_desc);

        self.pipeline_descs.len() - 1
    }

    pub fn prepare(&mut self, device: &crate::Device, renderer: &crate::Renderer) {
        puffin::profile_function!();
        // Load resources

        // Compile shaders using multithreading
        for (i, desc) in self.pipeline_descs.iter().enumerate() {
            // Todo: perhaps use Hash instead
            // Todo: this is the place to support shader recompilation
            if self.resources.pipelines.len() <= i {
                self.resources.pipelines.push(crate::Pipeline::new(
                    &device.handle,
                    desc.clone(),
                    Some(renderer.bindless_descriptor_set_layout),
                ));
            }
        }

        for pass in &mut self.passes {
            pass.try_create_read_texture_descriptor_set(
                device,
                &self.resources.pipelines,
                &self.resources.textures,
            );
            pass.try_create_uniform_buffer_descriptor_set(
                device,
                &self.resources.pipelines,
                &self.resources.buffers,
            );

            // Todo: free descriptor sets

            pass.update_uniform_buffer_memory(device, &mut self.resources.buffers);
        }
    }

    pub fn recompile_shaders(
        &mut self,
        device: &crate::Device,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) {
        for pass in &mut self.passes {
            self.resources.pipelines[pass.pipeline_handle]
                .recreate_pipeline(device, bindless_descriptor_set_layout);
        }
    }

    pub fn render(
        &mut self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        renderer: &Renderer,
        present_image: &[Image], // Todo: pass single value
    ) {
        puffin::profile_function!();

        for pass in &self.passes {
            let name = std::ffi::CString::new(pass.name.as_str()).unwrap();
            let debug_label = vk::DebugUtilsLabelEXT::builder()
                .label_name(&name)
                //.color([1.0, 0.0, 0.0, 1.0])
                .build();
            unsafe {
                device
                    .debug_utils
                    .cmd_begin_debug_utils_label(command_buffer, &debug_label)
            };

            // Transition pass resources
            // Todo: probably can combine reads and writes to one vector
            for read in &pass.reads {
                let next_access = crate::synch::image_pipeline_barrier(
                    &device,
                    command_buffer,
                    &self.resources.textures[*read].texture.image,
                    self.resources.textures[*read].prev_access,
                    vk_sync::AccessType::AnyShaderReadSampledImageOrUniformTexelBuffer,
                );

                self.resources.textures.get_mut(*read).unwrap().prev_access = next_access;
            }

            let mut writes_for_synch = pass.writes.clone();
            // If the depth attachment is owned by the graph make sure it gets a barrier as well
            if pass.depth_attachment.is_some() {
                if let DepthAttachment::GraphTexture(depth_attachment) =
                    pass.depth_attachment.as_ref().unwrap()
                {
                    writes_for_synch.push(*depth_attachment);
                }
            }

            for write in &writes_for_synch {
                let next_access = crate::synch::image_pipeline_barrier(
                    &device,
                    command_buffer,
                    &self.resources.textures[write.texture].texture.image,
                    self.resources.textures[write.texture].prev_access,
                    if Image::is_depth_image_fmt(
                        self.resources.textures[write.texture]
                            .texture
                            .image
                            .desc
                            .format,
                    ) {
                        vk_sync::AccessType::DepthStencilAttachmentWrite
                    } else {
                        vk_sync::AccessType::ColorAttachmentWrite
                    },
                );

                self.resources
                    .textures
                    .get_mut(write.texture)
                    .unwrap()
                    .prev_access = next_access;
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
                .map(|write| self.resources.textures[write.texture].texture.image.clone())
                .collect();

            // Todo: very ugly just to get the extents...
            let extent = if pass.writes.len() > 0 {
                vk::Extent2D {
                    width: self.resources.textures[pass.writes[0].texture]
                        .texture
                        .image
                        .width(),
                    height: self.resources.textures[pass.writes[0].texture]
                        .texture
                        .image
                        .height(),
                }
            } else {
                if pass.depth_attachment.is_some() {
                    match pass.depth_attachment.as_ref().unwrap() {
                        DepthAttachment::GraphTexture(depth_attachment) => vk::Extent2D {
                            width: self.resources.textures[depth_attachment.texture]
                                .texture
                                .image
                                .width(),
                            height: self.resources.textures[depth_attachment.texture]
                                .texture
                                .image
                                .height(),
                        },
                        DepthAttachment::External(depth_attachment) => vk::Extent2D {
                            width: depth_attachment.width(),
                            height: depth_attachment.height(),
                        },
                    }
                } else {
                    vk::Extent2D {
                        width: 1,
                        height: 1,
                    }
                }
            };

            pass.prepare_render(
                device,
                command_buffer,
                if !pass.presentation_pass {
                    write_attachments.as_slice()
                } else {
                    present_image
                },
                // Todo: ugly just to get the different types of depth attachments
                if pass.depth_attachment.is_some() {
                    match pass.depth_attachment.as_ref().unwrap() {
                        DepthAttachment::GraphTexture(depth_attachment) => Some((
                            self.resources.textures[depth_attachment.texture]
                                .texture
                                .image
                                .clone(),
                            depth_attachment.view,
                        )),
                        DepthAttachment::External(depth_attachment) => {
                            Some((depth_attachment.clone(), ViewType::Full()))
                        }
                    }
                } else {
                    None
                },
                if !pass.presentation_pass {
                    extent
                } else {
                    vk::Extent2D {
                        width: present_image[0].width(),   // Todo
                        height: present_image[0].height(), // Todo
                    }
                },
                &self.resources.pipelines,
            );

            // Todo: this could be moved outside the pass loop
            unsafe {
                device.handle.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.resources.pipelines[pass.pipeline_handle].pipeline_layout,
                    crate::DESCRIPTOR_SET_INDEX_VIEW,
                    &[self.descriptor_set_camera.handle],
                    &[],
                )
            };

            if let Some(read_textures_descriptor_set) = &pass.read_textures_descriptor_set {
                unsafe {
                    device.handle.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.resources.pipelines[pass.pipeline_handle].pipeline_layout,
                        crate::DESCRIPTOR_SET_INDEX_INPUT_TEXTURES,
                        &[read_textures_descriptor_set.handle],
                        &[],
                    )
                };
            }

            if let Some(uniforms_descriptor_set) = &pass.uniforms_descriptor_set {
                unsafe {
                    device.handle.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.resources.pipelines[pass.pipeline_handle].pipeline_layout,
                        self.resources.pipelines[pass.pipeline_handle]
                            .reflection
                            .get_binding(pass.uniforms.keys().next().unwrap())
                            .set,
                        &[uniforms_descriptor_set.handle],
                        &[],
                    )
                };
            }

            if let Some(render_func) = &pass.render_func {
                puffin::profile_scope!("render_func:", pass.name.as_str());
                render_func(device, command_buffer, renderer, pass, &self.resources);
            }

            unsafe {
                device.handle.cmd_end_rendering(command_buffer);
                device.debug_utils.cmd_end_debug_utils_label(command_buffer);
            }
        }
    }
}
