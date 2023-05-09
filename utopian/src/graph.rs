use std::collections::HashMap;
use std::mem::MaybeUninit;

use ash::vk;

use crate::device::*;
use crate::image::*;
use crate::pipeline::*;
use crate::Buffer;
use crate::Pipeline;
use crate::PipelineDesc;
use crate::RenderPass;
use crate::Renderer;
use crate::Texture;

/// Virtual resource handles.
///
/// Plain indexes into the graph's resource arrays.
pub type TextureId = usize;
pub type BufferId = usize;
pub type PipelineId = usize;
pub type TlasId = usize;

/// Texture owned by the graph.
pub struct GraphTexture {
    pub texture: Texture,
    pub prev_access: vk_sync::AccessType,
}

/// Buffer owned by the graph.
pub struct GraphBuffer {
    pub buffer: Buffer,
    pub prev_access: vk_sync::AccessType,
}

/// Resources owned by the graph.
///
/// This is what enables the resource caching.
/// Note: the resources are never cleared.
pub struct GraphResources {
    pub buffers: Vec<GraphBuffer>,
    pub textures: Vec<GraphTexture>,
    pub pipelines: Vec<Pipeline>,
}

/// Either an external or graph owned depth attachment.
pub enum DepthAttachment {
    GraphHandle(Attachment),
    External(Image, vk::AttachmentLoadOp),
}

#[derive(Copy, Clone)]
pub enum ViewType {
    Full(),
    Layer(u32),
}

#[derive(Copy, Clone)]
pub struct Attachment {
    pub texture: TextureId,
    pub view: ViewType,
    pub load_op: vk::AttachmentLoadOp,
}

#[derive(Copy, Clone, PartialEq)]
pub enum TextureResourceType {
    CombinedImageSampler,
    StorageImage,
}

#[derive(Copy, Clone)]
pub struct TextureResource {
    pub texture: TextureId,
    pub input_type: TextureResourceType,
    pub access_type: vk_sync::AccessType,
}

#[derive(Copy, Clone)]
pub struct BufferResource {
    pub buffer: BufferId,
    pub access_type: vk_sync::AccessType,
}

#[derive(Copy, Clone)]
pub enum Resource {
    Texture(TextureResource),
    Buffer(BufferResource),
    Tlas(TlasId),
}

/// Used in the `PassBuilder::copy_image` method.
pub struct TextureCopy {
    pub src: TextureId,
    pub dst: TextureId,
    pub copy_desc: vk::ImageCopy,
}

/// The frame graph that holds all the passes and resources.
pub struct Graph {
    pub passes: Vec<RenderPass>,
    pub resources: GraphResources,
    pub descriptor_set_camera: crate::DescriptorSet,
    pub pipeline_descs: Vec<PipelineDesc>,
    pub profiling_enabled: bool,
}

pub const MAX_UNIFORMS_SIZE: usize = 2048;

// Note: the way that the uniform data array is hardcoded in size might be a problem.
// The goal was to have a clear method for each pass to own its own data without adding complexity.
// A good improvement would be to use Dynamic Descriptor sets so that all data from the different
// passes is placed in the same buffer but with different offsets.
#[derive(Copy, Clone)]
pub struct UniformData {
    pub data: [MaybeUninit<u8>; MAX_UNIFORMS_SIZE],
    pub size: u64,
}

pub struct PassBuilder {
    pub name: String,
    pub pipeline_handle: PipelineId,
    pub reads: Vec<Resource>,
    pub writes: Vec<Attachment>,
    pub render_func:
        Option<Box<dyn Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass, &GraphResources)>>,
    pub depth_attachment: Option<DepthAttachment>,
    pub presentation_pass: bool,
    // The key is the uniform name with the pass name as prefix
    pub uniforms: HashMap<String, (String, UniformData)>,
    pub copy_command: Option<TextureCopy>,
    pub extra_barriers: Option<Vec<(BufferId, vk_sync::AccessType)>>,
}

impl PassBuilder {
    pub fn read(mut self, resource_id: TextureId) -> Self {
        self.reads.push(Resource::Texture(TextureResource {
            texture: resource_id,
            input_type: TextureResourceType::CombinedImageSampler,
            access_type: vk_sync::AccessType::AnyShaderReadSampledImageOrUniformTexelBuffer,
        }));
        self
    }

    pub fn image_write(mut self, resource_id: TextureId) -> Self {
        self.reads.push(Resource::Texture(TextureResource {
            texture: resource_id,
            input_type: TextureResourceType::StorageImage,
            access_type: vk_sync::AccessType::AnyShaderWrite,
        }));
        self
    }

    pub fn write_buffer(mut self, resource_id: BufferId) -> Self {
        self.reads.push(Resource::Buffer(BufferResource {
            buffer: resource_id,
            access_type: vk_sync::AccessType::AnyShaderWrite,
        }));
        self
    }

    pub fn read_buffer(mut self, resource_id: BufferId) -> Self {
        self.reads.push(Resource::Buffer(BufferResource {
            buffer: resource_id,
            access_type: vk_sync::AccessType::AnyShaderReadOther,
        }));
        self
    }

    pub fn write(mut self, resource_id: TextureId) -> Self {
        self.writes.push(Attachment {
            texture: resource_id,
            view: ViewType::Full(),
            load_op: vk::AttachmentLoadOp::CLEAR,
        });
        self
    }

    pub fn write_layer(mut self, resource_id: TextureId, layer: u32) -> Self {
        self.writes.push(Attachment {
            texture: resource_id,
            view: ViewType::Layer(layer),
            load_op: vk::AttachmentLoadOp::CLEAR,
        });
        self
    }

    pub fn load_write(mut self, resource_id: TextureId) -> Self {
        self.writes.push(Attachment {
            texture: resource_id,
            view: ViewType::Full(),
            load_op: vk::AttachmentLoadOp::LOAD,
        });
        self
    }

    pub fn tlas(mut self, resource_id: TlasId) -> Self {
        // The acceleration structure is specially treated for now since it is
        // an external resource not owned by the graph. The passed resource_id
        // is unused.
        self.reads.push(Resource::Tlas(resource_id));
        self
    }

    /// Allows for adding extra buffer barriers for resources that the other APIs don't cover.
    pub fn extra_barriers(mut self, buffers: &[(BufferId, vk_sync::AccessType)]) -> Self {
        self.extra_barriers = Some(buffers.to_vec());
        self
    }

    /// Records custom rendering commands to the command buffer.
    pub fn render(
        mut self,
        render_func: impl Fn(&Device, vk::CommandBuffer, &Renderer, &RenderPass, &GraphResources)
            + 'static,
    ) -> Self {
        self.render_func.replace(Box::new(render_func));
        self
    }

    /// Convenience function for dispatching compute shaders.
    pub fn dispatch(mut self, group_count_x: u32, group_count_y: u32, group_count_z: u32) -> Self {
        self.render_func
            .replace(Box::new(move |device, command_buffer, _, _, _| unsafe {
                device.handle.cmd_dispatch(
                    command_buffer,
                    group_count_x,
                    group_count_y,
                    group_count_z,
                );
            }));
        self
    }

    /// Convenience function for tracing rays.
    pub fn trace_rays(mut self, width: u32, height: u32, depth: u32) -> Self {
        self.render_func.replace(Box::new(
            move |device, command_buffer, _, pass, resources| unsafe {
                let pipeline = resources.pipeline(pass.pipeline_handle);
                if let Some(raytracing_sbt) = &pipeline.raytracing_sbt {
                    device.raytracing_pipeline_ext.cmd_trace_rays(
                        command_buffer,
                        &raytracing_sbt.raygen_sbt,
                        &raytracing_sbt.miss_sbt,
                        &raytracing_sbt.hit_sbt,
                        &raytracing_sbt.callable_sbt,
                        width,
                        height,
                        depth,
                    );
                } else {
                    panic!("No raytracing SBT found");
                }
            },
        ));
        self
    }

    /// Creates a copy commmand that executes after the `Pass::render_func` has finished.
    pub fn copy_image(mut self, src: TextureId, dst: TextureId, copy_desc: vk::ImageCopy) -> Self {
        self.copy_command.replace(TextureCopy {
            src,
            dst,
            copy_desc,
        });
        self
    }

    /// Specify this pass to use the current frames `VulkanBase::present_images` as color attachment
    pub fn presentation_pass(mut self, is_presentation_pass: bool) -> Self {
        self.presentation_pass = is_presentation_pass;
        self
    }

    /// Use image as depth attachment.
    pub fn depth_attachment(mut self, depth_attachment: TextureId) -> Self {
        self.depth_attachment = Some(DepthAttachment::GraphHandle(Attachment {
            texture: depth_attachment,
            view: ViewType::Full(),
            load_op: vk::AttachmentLoadOp::CLEAR, // Todo
        }));
        self
    }

    /// Use a specific layer of an image as depth attachment.
    pub fn depth_attachment_layer(mut self, depth_attachment: TextureId, layer: u32) -> Self {
        self.depth_attachment = Some(DepthAttachment::GraphHandle(Attachment {
            texture: depth_attachment,
            view: ViewType::Layer(layer),
            load_op: vk::AttachmentLoadOp::CLEAR, // Todo
        }));
        self
    }

    /// Use an external depth attachment that is not owned by the graph.
    pub fn external_depth_attachment(
        mut self,
        depth_attachment: Image,
        load_op: vk::AttachmentLoadOp,
    ) -> Self {
        self.depth_attachment = Some(DepthAttachment::External(depth_attachment, load_op));
        self
    }

    /// Constant uniform data that is passed to the shader.
    pub fn uniforms<T: Copy + std::fmt::Debug>(mut self, name: &str, data: &T) -> Self {
        puffin::profile_function!();

        // Note: Todo: this can be improved
        unsafe {
            let ptr = data as *const _ as *const MaybeUninit<u8>;
            let size = std::mem::size_of::<T>();
            let data_u8 = std::slice::from_raw_parts(ptr, size);

            assert!(data_u8.len() < MAX_UNIFORMS_SIZE);

            // Pass name + uniform name
            let unique_name = self.name.clone() + "_" + name;

            if let Some(entry) = self.uniforms.get_mut(&unique_name) {
                entry.1.data[..data_u8.len()].copy_from_slice(data_u8);
                entry.1.size = size as u64;
            } else {
                let mut new_entry = UniformData {
                    data: [MaybeUninit::zeroed(); MAX_UNIFORMS_SIZE],
                    size: size as u64,
                };
                new_entry.data[..data_u8.len()].copy_from_slice(data_u8);
                self.uniforms
                    .insert(unique_name.to_string(), (name.to_string(), new_entry));
            }
        }
        self
    }

    /// Creates a new pass and adds it to the graph.
    ///
    /// Also updates the color attachment formats of the pipeline since
    /// they at this stage are known.
    /// Note: allocates and create the constant uniform buffer if not cached.
    pub fn build(self, device: &Device, graph: &mut Graph) {
        puffin::profile_function!();

        let mut pass = crate::RenderPass::new(
            self.name,
            self.pipeline_handle,
            self.presentation_pass,
            self.depth_attachment,
            self.uniforms.clone(), // Note: is this clone OK?
            self.render_func,
            self.copy_command,
            self.extra_barriers,
        );

        for read in &self.reads {
            pass.reads.push(*read);
        }

        for write in &self.writes {
            pass.writes.push(*write);
        }

        // Update attachment formats now that all writes are known
        graph.pipeline_descs[pass.pipeline_handle].color_attachment_formats = pass
            .writes
            .iter()
            .map(|write| {
                graph
                    .resources
                    .texture(write.texture)
                    .texture
                    .image
                    .format()
            })
            .collect();

        if let Some(depth) = &pass.depth_attachment {
            match depth {
                DepthAttachment::GraphHandle(write) => {
                    graph.pipeline_descs[pass.pipeline_handle].depth_stencil_attachment_format =
                        graph
                            .resources
                            .texture(write.texture)
                            .texture
                            .image
                            .format()
                }
                DepthAttachment::External(image, _) => {
                    graph.pipeline_descs[pass.pipeline_handle].depth_stencil_attachment_format =
                        image.format()
                }
            }
        }

        if self.uniforms.len() != 0 {
            pass.uniform_buffer.replace(graph.create_buffer(
                &self.uniforms.keys().next().unwrap(),
                device,
                self.uniforms.values().next().unwrap().1.size as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                gpu_allocator::MemoryLocation::CpuToGpu,
            ));
        }

        graph.passes.push(pass);
    }
}

impl GraphResources {
    fn new() -> GraphResources {
        GraphResources {
            buffers: vec![],
            textures: vec![],
            pipelines: vec![],
        }
    }

    pub fn buffer<'a>(&'a self, id: BufferId) -> &'a GraphBuffer {
        &self.buffers[id]
    }

    pub fn texture<'a>(&'a self, id: TextureId) -> &'a GraphTexture {
        &self.textures[id]
    }

    pub fn pipeline<'a>(&'a self, id: PipelineId) -> &'a Pipeline {
        &self.pipelines[id]
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
            profiling_enabled: false,
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
            if let Some(descriptor_set) = &pass.read_resources_descriptor_set {
                unsafe {
                    device
                        .handle
                        .destroy_descriptor_pool(descriptor_set.pool, None)
                };
            }
        }

        self.passes.clear();
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
            copy_command: None,
            extra_barriers: None,
        }
    }

    pub fn add_pass_from_desc(
        &mut self,
        name: &str,
        desc_builder: PipelineDescBuilder,
    ) -> PassBuilder {
        let pipeline_handle = self.create_pipeline(desc_builder.build());

        self.add_pass(name.to_string(), pipeline_handle)
    }

    /// Creates a texture and returns its handle.
    ///
    /// If a texture with the same name already exists, it will be returned instead.
    /// Compared to `graph::create_pipeline`, this function does not defer the creation of the texture.
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
            .position(|iter| iter.texture.image.debug_name == debug_name)
            .unwrap_or_else(|| {
                let texture = crate::Texture::create(&device, None, image_desc, debug_name);

                self.resources.textures.push(GraphTexture {
                    texture,
                    prev_access: vk_sync::AccessType::Nothing,
                });

                self.resources.textures.len() - 1
            })
    }

    /// Creates a buffer and returns its handle.
    ///
    /// If a buffer with the same name already exists, it will be returned instead.
    /// Compared to `graph::create_pipeline`, this function does not defer the creation of the buffer.
    pub fn create_buffer(
        &mut self,
        debug_name: &str,
        device: &crate::Device,
        size: u64,
        usage: vk::BufferUsageFlags,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> BufferId {
        puffin::profile_function!();

        self.resources
            .buffers
            .iter()
            .position(|iter| iter.buffer.debug_name == debug_name)
            .unwrap_or_else(|| {
                let mut buffer = Buffer::new::<u8>(device, None, size, usage, memory_location);

                buffer.set_debug_name(device, debug_name);

                self.resources.buffers.push(GraphBuffer {
                    buffer,
                    prev_access: vk_sync::AccessType::Nothing,
                });

                self.resources.buffers.len() - 1
            })
    }

    /// Creates a pipeline and returns its handle.
    ///
    /// The pipeline creation is deferred until the `graph::prepare` function is called.
    pub fn create_pipeline(&mut self, pipeline_desc: PipelineDesc) -> PipelineId {
        if let Some(existing_pipeline_id) = self
            .pipeline_descs
            .iter()
            .position(|desc| *desc == pipeline_desc)
        {
            existing_pipeline_id
        } else {
            self.pipeline_descs.push(pipeline_desc);
            self.pipeline_descs.len() - 1
        }
    }

    pub fn prepare(&mut self, device: &crate::Device, renderer: &crate::Renderer) {
        puffin::profile_function!();

        // Todo: shall be possible to create the pipelines using multiple threads
        for (i, desc) in self.pipeline_descs.iter().enumerate() {
            if self.resources.pipelines.len() <= i {
                self.resources.pipelines.push(crate::Pipeline::new(
                    device,
                    desc.clone(),
                    Some(renderer.bindless_descriptor_set_layout),
                ));
            }
        }

        for pass in &mut self.passes {
            pass.try_create_read_resources_descriptor_set(
                device,
                &self.resources.pipelines,
                &self.resources.textures,
                &self.resources.buffers,
                if let Some(raytracing) = &renderer.raytracing {
                    raytracing.top_level_acceleration.as_ref().unwrap().handle
                } else {
                    vk::AccelerationStructureKHR::null()
                },
            );
            pass.try_create_uniform_buffer_descriptor_set(
                device,
                &self.resources.pipelines,
                &self.resources.buffers,
            );

            pass.update_uniform_buffer_memory(device, &mut self.resources.buffers);
        }
    }

    pub fn recompile_all_shaders(
        &mut self,
        device: &crate::Device,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) {
        for pipeline in &mut self.resources.pipelines {
            pipeline.recreate_pipeline(device, bindless_descriptor_set_layout);
        }
    }

    pub fn recompile_shader(
        &mut self,
        device: &crate::Device,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
        path: std::path::PathBuf,
    ) {
        for pipeline in &mut self.resources.pipelines {
            let desc = &pipeline.pipeline_desc;
            if desc.compute_path.map_or(false, |p| path.ends_with(&p))
                || desc.vertex_path.map_or(false, |p| path.ends_with(&p))
                || desc.fragment_path.map_or(false, |p| path.ends_with(&p))
            {
                pipeline.recreate_pipeline(device, bindless_descriptor_set_layout);
            }
        }
    }

    pub fn render(
        &mut self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        renderer: &mut Renderer,
        present_image: &[Image], // Todo: pass single value
        rebuild_tlas: bool,
    ) {
        puffin::profile_function!();

        self.begin_gpu_profiler_frame(device, command_buffer);

        if let Some(raytracing) = &mut renderer.raytracing {
            if rebuild_tlas {
                let rebuild_tlas_scope = self.begin_gpu_scope(
                    device,
                    command_buffer,
                    &String::from("rebuild_tlas_pass"),
                );

                crate::synch::global_pipeline_barrier(
                    device,
                    command_buffer,
                    vk_sync::AccessType::AnyShaderReadOther,
                    vk_sync::AccessType::AccelerationStructureBuildWrite,
                );

                raytracing.rebuild_tlas(device, command_buffer, &renderer.instances);

                crate::synch::global_pipeline_barrier(
                    device,
                    command_buffer,
                    vk_sync::AccessType::AccelerationStructureBuildWrite,
                    vk_sync::AccessType::AnyShaderReadOther,
                );

                self.end_gpu_scope(device, command_buffer, rebuild_tlas_scope);
            }
        }

        for pass in &self.passes {
            let active_gpu_scope = self.begin_gpu_scope(device, command_buffer, &pass.name);

            let pass_pipeline = &self.resources.pipelines[pass.pipeline_handle];

            // Transition pass resources
            // Todo: probably can combine reads and writes to one vector
            for read in &pass.reads {
                // Todo: also add buffer barriers
                // https://themaister.net/blog/2019/08/14/yet-another-blog-explaining-vulkan-synchronization/:
                // "This is not very interesting, we’re just restricting memory availability and visibility to a specific buffer.
                // No GPU I know of actually cares, I think it makes more sense to just use VkMemoryBarrier rather than
                // bothering with buffer barriers.

                match read {
                    Resource::Texture(read) => {
                        let next_access = crate::synch::image_pipeline_barrier(
                            &device,
                            command_buffer,
                            &self.resources.textures[read.texture].texture.image,
                            self.resources.textures[read.texture].prev_access,
                            read.access_type,
                        );

                        self.resources
                            .textures
                            .get_mut(read.texture)
                            .unwrap()
                            .prev_access = next_access;
                    }

                    Resource::Buffer(read) => {
                        let next_access = crate::synch::global_pipeline_barrier(
                            device,
                            command_buffer,
                            self.resources.buffers[read.buffer].prev_access,
                            read.access_type,
                        );

                        self.resources
                            .buffers
                            .get_mut(read.buffer)
                            .unwrap()
                            .prev_access = next_access;
                    }
                    Resource::Tlas(_) => { // No resource transition for now (static)
                    }
                }
            }

            if let Some(extra_barriers) = &pass.extra_barriers {
                for (buffer_id, access_type) in extra_barriers {
                    let next_access = crate::synch::global_pipeline_barrier(
                        device,
                        command_buffer,
                        self.resources.buffers[*buffer_id].prev_access,
                        *access_type,
                    );

                    // Update the buffer's previous access type with the next access type
                    if let Some(buffer) = self.resources.buffers.get_mut(*buffer_id) {
                        buffer.prev_access = next_access;
                    }
                }
            }

            let mut writes_for_synch = pass.writes.clone();
            // If the depth attachment is owned by the graph make sure it gets a barrier as well
            if pass.depth_attachment.is_some() {
                if let DepthAttachment::GraphHandle(depth_attachment) =
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

            let write_attachments: Vec<(Image, ViewType, vk::AttachmentLoadOp)> = pass
                .writes
                .iter()
                .map(|write| {
                    (
                        self.resources.textures[write.texture].texture.image.clone(),
                        write.view,
                        write.load_op,
                    )
                })
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
                        DepthAttachment::GraphHandle(depth_attachment) => vk::Extent2D {
                            width: self.resources.textures[depth_attachment.texture]
                                .texture
                                .image
                                .width(),
                            height: self.resources.textures[depth_attachment.texture]
                                .texture
                                .image
                                .height(),
                        },
                        DepthAttachment::External(depth_attachment, _) => vk::Extent2D {
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

            assert_eq!(present_image.len(), 1);
            let present_image = [(
                present_image[0].clone(),
                ViewType::Full(),
                vk::AttachmentLoadOp::CLEAR,
            )];

            pass.prepare_render(
                device,
                command_buffer,
                if !pass.presentation_pass {
                    write_attachments.as_slice()
                } else {
                    &present_image
                },
                // Todo: ugly just to get the different types of depth attachments
                if pass.depth_attachment.is_some() {
                    match pass.depth_attachment.as_ref().unwrap() {
                        DepthAttachment::GraphHandle(depth_attachment) => Some((
                            self.resources.textures[depth_attachment.texture]
                                .texture
                                .image
                                .clone(),
                            depth_attachment.view,
                            depth_attachment.load_op,
                        )),
                        DepthAttachment::External(depth_attachment, load_op) => {
                            Some((depth_attachment.clone(), ViewType::Full(), *load_op))
                        }
                    }
                } else {
                    None
                },
                if !pass.presentation_pass {
                    extent
                } else {
                    vk::Extent2D {
                        width: present_image[0].0.width(),   // Todo
                        height: present_image[0].0.height(), // Todo
                    }
                },
                &self.resources.pipelines,
            );

            // Bind descriptor sets that are used by all passes.
            // This includes bindless resources, view data, input textures
            // and uniform buffers from each pass with constants.
            unsafe {
                let bind_point = match pass_pipeline.pipeline_type {
                    PipelineType::Graphics => vk::PipelineBindPoint::GRAPHICS,
                    PipelineType::Compute => vk::PipelineBindPoint::COMPUTE,
                    PipelineType::Raytracing => vk::PipelineBindPoint::RAY_TRACING_KHR,
                };

                device.handle.cmd_bind_descriptor_sets(
                    command_buffer,
                    bind_point,
                    pass_pipeline.pipeline_layout,
                    crate::DESCRIPTOR_SET_INDEX_BINDLESS,
                    &[renderer.bindless_descriptor_set],
                    &[],
                );

                device.handle.cmd_bind_descriptor_sets(
                    command_buffer,
                    bind_point,
                    pass_pipeline.pipeline_layout,
                    crate::DESCRIPTOR_SET_INDEX_VIEW,
                    &[self.descriptor_set_camera.handle],
                    &[],
                );

                if let Some(read_textures_descriptor_set) = &pass.read_resources_descriptor_set {
                    device.handle.cmd_bind_descriptor_sets(
                        command_buffer,
                        bind_point,
                        pass_pipeline.pipeline_layout,
                        crate::DESCRIPTOR_SET_INDEX_INPUT_TEXTURES,
                        &[read_textures_descriptor_set.handle],
                        &[],
                    )
                }

                if let Some(uniforms_descriptor_set) = &pass.uniforms_descriptor_set {
                    device.handle.cmd_bind_descriptor_sets(
                        command_buffer,
                        bind_point,
                        pass_pipeline.pipeline_layout,
                        pass_pipeline
                            .reflection
                            .get_binding(&pass.uniforms.values().next().unwrap().0)
                            .set,
                        &[uniforms_descriptor_set.handle],
                        &[],
                    )
                }
            };

            if let Some(render_func) = &pass.render_func {
                puffin::profile_scope!("render_func:", pass.name.as_str());
                render_func(device, command_buffer, renderer, pass, &self.resources);
            }

            if pass_pipeline.pipeline_type == PipelineType::Graphics {
                unsafe { device.handle.cmd_end_rendering(command_buffer) };
            }

            if let Some(copy_command) = &pass.copy_command {
                puffin::profile_scope!("copy_command:", pass.name.as_str());

                let src = copy_command.src;
                let dst = copy_command.dst;

                // Image barriers
                // (a bit verbose, but ok for now)
                let next_access = crate::synch::image_pipeline_barrier(
                    &device,
                    command_buffer,
                    &self.resources.textures[src].texture.image,
                    self.resources.textures[src].prev_access,
                    vk_sync::AccessType::TransferRead,
                );
                self.resources.textures.get_mut(src).unwrap().prev_access = next_access;

                let next_access = crate::synch::image_pipeline_barrier(
                    &device,
                    command_buffer,
                    &self.resources.textures[dst].texture.image,
                    self.resources.textures[dst].prev_access,
                    vk_sync::AccessType::TransferWrite,
                );
                self.resources.textures.get_mut(dst).unwrap().prev_access = next_access;

                let src = &self.resources.textures[src].texture.image;
                let dst = &self.resources.textures[dst].texture.image;

                // Use aspect flags from images
                let mut copy_desc = copy_command.copy_desc;
                copy_desc.src_subresource.aspect_mask = src.desc.aspect_flags;
                copy_desc.dst_subresource.aspect_mask = dst.desc.aspect_flags;

                unsafe {
                    device.handle.cmd_copy_image(
                        command_buffer,
                        src.image,
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                        dst.image,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &[copy_desc],
                    )
                };
            }

            self.end_gpu_scope(device, command_buffer, active_gpu_scope);
        }

        self.end_gpu_profiler_frame(device, command_buffer);
    }

    pub fn begin_gpu_profiler_frame(&self, device: &Device, command_buffer: vk::CommandBuffer) {
        if self.profiling_enabled {
            device
                .frame_profiler
                .begin_frame(&device.handle, command_buffer);
        }
    }

    pub fn end_gpu_profiler_frame(&self, device: &Device, command_buffer: vk::CommandBuffer) {
        if self.profiling_enabled {
            device
                .frame_profiler
                .end_frame(&device.handle, command_buffer);
        }
    }

    pub fn begin_gpu_scope(
        &self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        name: &String,
    ) -> Option<gpu_profiler::backend::ash::VulkanActiveScope> {
        if !self.profiling_enabled {
            return None;
        }

        let name_c_str = std::ffi::CString::new(name.as_str()).unwrap();
        let debug_label = vk::DebugUtilsLabelEXT::builder()
            .label_name(&name_c_str)
            //.color([1.0, 0.0, 0.0, 1.0])
            .build();
        unsafe {
            device
                .debug_utils
                .cmd_begin_debug_utils_label(command_buffer, &debug_label)
        };

        let active_scope = {
            let query_id = gpu_profiler::profiler().create_scope(name);
            device
                .frame_profiler
                .begin_scope(&device.handle, command_buffer, query_id)
        };

        Some(active_scope)
    }

    pub fn end_gpu_scope(
        &self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        active_scope: Option<gpu_profiler::backend::ash::VulkanActiveScope>,
    ) {
        if self.profiling_enabled {
            device
                .frame_profiler
                .end_scope(&device.handle, command_buffer, active_scope.unwrap());

            unsafe { device.debug_utils.cmd_end_debug_utils_label(command_buffer) };
        }
    }
}
