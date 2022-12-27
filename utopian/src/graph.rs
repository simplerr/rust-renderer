use std::collections::HashMap;

use ash::vk;

use crate::descriptor_set::DescriptorIdentifier;
use crate::device::*;
use crate::image::*;
use crate::Pipeline;
use crate::PipelineDesc;
use crate::RenderPass;
use crate::Renderer;
use crate::Texture;

pub type TextureId = usize;
pub type BufferId = usize;
pub type PipelineId = usize;

pub const MAX_UNIFORMS_SIZE: usize = 2048;

pub struct GraphResource {
    pub texture: Texture,
    pub prev_access: vk_sync::AccessType,
}

pub struct Graph {
    pub passes: Vec<RenderPass>,
    pub resources: Vec<GraphResource>,
    pub buffers: Vec<crate::Buffer>,
    pub descriptor_set_camera: crate::DescriptorSet,
    pub pipeline_descs: Vec<PipelineDesc>,
    pub pipelines: Vec<Pipeline>,
}

#[derive(Copy, Clone)]
pub struct UniformData {
    pub data: [u8; MAX_UNIFORMS_SIZE],
    pub size: u64,
}

pub struct PassBuilder {
    pub name: String,
    pub pipeline_handle: PipelineId,
    pub reads: Vec<TextureId>,
    pub writes: Vec<TextureId>,
    pub render_func:
        Option<Box<dyn Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass, &Vec<Pipeline>)>>,
    pub depth_attachment: Option<Image>,
    pub presentation_pass: bool,
    pub uniforms: HashMap<String, UniformData>,
}

impl PassBuilder {
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
        render_func: impl Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass, &Vec<Pipeline>)
            + 'static,
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

    pub fn uniforms<T: Copy + std::fmt::Debug>(mut self, name: &str, data: &T) -> Self {
        puffin::profile_function!();

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
    pub fn new(device: &Device, camera_uniform_buffer: &crate::Buffer) -> Self {
        Graph {
            passes: vec![],
            resources: vec![],
            buffers: vec![],
            descriptor_set_camera: Self::create_camera_descriptor_set(
                device,
                camera_uniform_buffer,
            ),
            pipelines: vec![],
            pipeline_descs: vec![],
        }
    }

    pub fn clear(&mut self) {
        self.passes.clear();
        self.pipeline_descs.clear();
    }

    pub fn create_camera_descriptor_set(
        device: &Device,
        camera_uniform_buffer: &crate::Buffer,
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
            //graph: self,
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
        extent: [u32; 2],
        format: vk::Format,
    ) -> TextureId {
        puffin::profile_function!();

        // Todo: Cannot rely on debug_name being unique
        // Todo: shall use a Hash to include extent and format of the texture
        self.resources
            .iter()
            .position(|iter| iter.texture.debug_name == debug_name)
            .unwrap_or_else(|| {
                let mut texture =
                    crate::Texture::create(&device, None, extent[0], extent[1], format);
                texture.set_debug_name(device, debug_name);

                self.resources.push(GraphResource {
                    texture,
                    prev_access: vk_sync::AccessType::Nothing,
                });

                self.resources.len() - 1
            })
    }

    pub fn create_buffer(
        &mut self,
        debug_name: &str,
        device: &crate::Device,
        size: u64,
    ) -> BufferId {
        // Todo: Cannot rely on debug_name being unique

        self.buffers
            .iter()
            .position(|iter| iter.debug_name == debug_name)
            .unwrap_or_else(|| {
                let mut buffer = crate::Buffer::new::<u8>(
                    device,
                    None,
                    size,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                    gpu_allocator::MemoryLocation::CpuToGpu,
                );

                buffer.set_debug_name(device, debug_name);

                self.buffers.push(buffer);

                self.buffers.len() - 1
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
            if self.pipelines.len() <= i {
                self.pipelines.push(crate::Pipeline::new(
                    &device.handle,
                    desc.clone(),
                    Some(renderer.bindless_descriptor_set_layout),
                ));
            }
        }

        for pass in &mut self.passes {
            pass.try_create_read_texture_descriptor_set(device, &self.pipelines, &self.resources);
            pass.try_create_uniform_buffer_descriptor_set(device, &self.pipelines, &self.buffers);

            // Todo: free descriptor sets

            pass.update_uniform_buffer_memory(device, &mut self.buffers);
        }
    }

    pub fn recompile_shaders(
        &mut self,
        device: &crate::Device,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) {
        for pass in &mut self.passes {
            self.pipelines[pass.pipeline_handle]
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
                &self.pipelines,
            );

            // Todo: this could be moved outside the pass loop
            unsafe {
                device.handle.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipelines[pass.pipeline_handle].pipeline_layout,
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
                        self.pipelines[pass.pipeline_handle].pipeline_layout,
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
                        self.pipelines[pass.pipeline_handle].pipeline_layout,
                        self.pipelines[pass.pipeline_handle]
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
                render_func(device, command_buffer, renderer, pass, &self.pipelines);
            }

            unsafe {
                device.handle.cmd_end_rendering(command_buffer);
                device.debug_utils.cmd_end_debug_utils_label(command_buffer);
            }
        }
    }
}
