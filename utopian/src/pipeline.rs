use ash::vk;
use std::ffi::CStr;
use std::hash::{Hash, Hasher};
use std::io::Cursor;

use crate::*;

#[derive(Clone)]
pub struct PipelineDesc {
    pub vertex_path: Option<&'static str>,
    pub fragment_path: Option<&'static str>,
    pub compute_path: Option<&'static str>,
    pub raygen_path: Option<&'static str>,
    pub miss_path: Option<&'static str>,
    pub hit_path: Option<&'static str>,
    pub vertex_input_binding_descriptions: Vec<vk::VertexInputBindingDescription>,
    pub vertex_input_attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
    pub color_attachment_formats: Vec<vk::Format>,
    pub depth_stencil_attachment_format: vk::Format,
}

pub struct PipelineDescBuilder {
    desc: PipelineDesc,
}

pub struct Pipeline {
    pub handle: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    pub reflection: shader::Reflection,
    pub pipeline_desc: PipelineDesc,
    pub pipeline_type: PipelineType,
    pub raytracing_sbt: Option<RayTracingSbt>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum PipelineType {
    Graphics,
    Compute,
    Raytracing,
}

#[allow(dead_code)]
pub struct RayTracingSbt {
    raygen_sbt_buffer: Buffer,
    miss_sbt_buffer: Buffer,
    hit_sbt_buffer: Buffer,
    pub raygen_sbt: vk::StridedDeviceAddressRegionKHR,
    pub miss_sbt: vk::StridedDeviceAddressRegionKHR,
    pub hit_sbt: vk::StridedDeviceAddressRegionKHR,
    pub callable_sbt: vk::StridedDeviceAddressRegionKHR,
}

impl Hash for PipelineDesc {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.vertex_path.hash(state);
        self.fragment_path.hash(state);
        self.color_attachment_formats.hash(state);
        self.depth_stencil_attachment_format.hash(state);
    }
}

impl PartialEq for PipelineDesc {
    fn eq(&self, other: &Self) -> bool {
        self.vertex_path == other.vertex_path
            && self.fragment_path == other.fragment_path
            && self.compute_path == other.compute_path
            && self.raygen_path == other.raygen_path
            && self.miss_path == other.miss_path
            && self.hit_path == other.hit_path
    }
}

impl Pipeline {
    pub fn new(
        device: &Device,
        pipeline_desc: PipelineDesc,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) -> Pipeline {
        puffin::profile_function!();

        let pipeline_type = match (&pipeline_desc.compute_path, &pipeline_desc.raygen_path) {
            (_, Some(_)) => PipelineType::Raytracing,
            (Some(_), _) => PipelineType::Compute,
            (None, None) => PipelineType::Graphics,
        };

        let mut pipeline = Pipeline {
            handle: vk::Pipeline::null(),
            pipeline_layout: vk::PipelineLayout::null(),
            descriptor_set_layouts: vec![],
            reflection: shader::Reflection::default(),
            pipeline_desc: pipeline_desc.clone(),
            pipeline_type,
            raytracing_sbt: None,
        };

        Self::create_pipeline(&mut pipeline, device, bindless_descriptor_set_layout)
            .expect("Error creating pipeline");

        pipeline
    }

    pub fn recreate_pipeline(
        &mut self,
        device: &Device,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) -> bool {
        // Todo: cleanup old resources

        if let Ok(_) = Self::create_pipeline(self, device, bindless_descriptor_set_layout) {
            println!("Successfully recompiled shader");
            return true;
        }
        false
    }

    fn create_pipeline(
        pipeline: &mut Pipeline,
        device: &Device,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) -> Result<(), ()> {
        let desc = &pipeline.pipeline_desc;
        let (shader_stage_create_infos, reflection, pipeline_layout, descriptor_set_layouts) =
            match pipeline.pipeline_type {
                PipelineType::Graphics => Pipeline::create_graphics_shader_modules(
                    &device.handle,
                    desc.vertex_path.unwrap(),
                    desc.fragment_path.unwrap(),
                    bindless_descriptor_set_layout,
                ),
                PipelineType::Compute => Pipeline::create_compute_shader_modules(
                    &device.handle,
                    desc.compute_path.unwrap(),
                    bindless_descriptor_set_layout,
                ),
                PipelineType::Raytracing => Pipeline::create_raytracing_shader_modules(
                    &device.handle,
                    desc.raygen_path.unwrap(),
                    desc.miss_path.unwrap(),
                    desc.hit_path.unwrap(),
                    bindless_descriptor_set_layout,
                ),
            }
            .map_err(|error| {
                println!("Failed to compile shader: {:#?}", error);
            })?;

        let new_handle = match pipeline.pipeline_type {
            PipelineType::Graphics => Pipeline::create_graphics_pipeline(
                &device.handle,
                shader_stage_create_infos,
                desc.color_attachment_formats.as_slice(),
                desc.depth_stencil_attachment_format,
                pipeline_layout,
                &pipeline.pipeline_desc,
            ),
            PipelineType::Compute => Pipeline::create_compute_pipeline(
                &device.handle,
                shader_stage_create_infos,
                pipeline_layout,
            ),
            PipelineType::Raytracing => Pipeline::create_raytracing_pipeline(
                &device,
                shader_stage_create_infos,
                pipeline_layout,
            ),
        };

        if pipeline.pipeline_type == PipelineType::Raytracing {
            let raytracing_sbt = Pipeline::create_raytracing_sbt(&device, new_handle);
            pipeline.raytracing_sbt = Some(raytracing_sbt);
        }

        pipeline.handle = new_handle;
        pipeline.pipeline_layout = pipeline_layout;
        pipeline.descriptor_set_layouts = descriptor_set_layouts;
        pipeline.reflection = reflection;

        Ok(())
    }

    fn create_graphics_shader_modules(
        device: &ash::Device,
        vertex_shader_path: &str,
        fragment_shader_path: &str,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) -> Result<
        (
            Vec<vk::PipelineShaderStageCreateInfo>,
            shader::Reflection,
            vk::PipelineLayout,
            Vec<vk::DescriptorSetLayout>,
        ),
        shaderc::Error,
    > {
        let vertex_spv_file = shader::compile_glsl_shader(vertex_shader_path)?;
        let fragment_spv_file = shader::compile_glsl_shader(fragment_shader_path)?;

        let vertex_spv_file = vertex_spv_file.as_binary_u8();
        let fragment_spv_file = fragment_spv_file.as_binary_u8();

        let reflection = shader::Reflection::new(&[vertex_spv_file, fragment_spv_file]);

        let (pipeline_layout, descriptor_set_layouts, _) = shader::create_layouts_from_reflection(
            device,
            &reflection,
            bindless_descriptor_set_layout,
        );

        let vertex_spv_file = Cursor::new(vertex_spv_file);
        let fragment_spv_file = Cursor::new(fragment_spv_file);

        let vertex_shader_module = shader::create_shader_module(vertex_spv_file, device);
        let fragment_shader_module = shader::create_shader_module(fragment_spv_file, device);

        let shader_entry_name = CStr::from_bytes_with_nul(b"main\0").unwrap();
        let shader_stage_create_infos = vec![
            vk::PipelineShaderStageCreateInfo {
                module: vertex_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: fragment_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];

        Ok((
            shader_stage_create_infos,
            reflection,
            pipeline_layout,
            descriptor_set_layouts,
        ))
    }

    fn create_graphics_pipeline(
        device: &ash::Device,
        shader_stage_create_infos: Vec<vk::PipelineShaderStageCreateInfo>,
        color_attachment_formats: &[vk::Format],
        depth_stencil_attachment_format: vk::Format,
        pipeline_layout: vk::PipelineLayout,
        pipeline_desc: &PipelineDesc,
    ) -> vk::Pipeline {
        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(
                pipeline_desc.vertex_input_attribute_descriptions.as_slice(),
            )
            .vertex_binding_descriptions(
                pipeline_desc.vertex_input_binding_descriptions.as_slice(),
            );
        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            ..Default::default()
        };
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            line_width: 1.0,
            polygon_mode: vk::PolygonMode::FILL,
            ..Default::default()
        };
        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        let noop_stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            ..Default::default()
        };
        let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: 1,
            depth_write_enable: 1,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
            front: noop_stencil_state,
            back: noop_stencil_state,
            max_depth_bounds: 1.0,
            ..Default::default()
        };
        let color_blend_attachment_states = vec![
            vk::PipelineColorBlendAttachmentState {
                blend_enable: 0,
                src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
                dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
                color_blend_op: vk::BlendOp::ADD,
                src_alpha_blend_factor: vk::BlendFactor::ZERO,
                dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                alpha_blend_op: vk::BlendOp::ADD,
                color_write_mask: vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            };
            color_attachment_formats.len() // Note: Todo: the attachments will in the future need different blend attachment states
        ];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);

        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_state);

        let mut rendering_info = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(color_attachment_formats)
            .depth_attachment_format(depth_stencil_attachment_format)
            .stencil_attachment_format(depth_stencil_attachment_format)
            .build();

        let graphic_pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_state_info)
            .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(vk::RenderPass::null())
            .push_next(&mut rendering_info);

        let graphics_pipelines = unsafe {
            device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[graphic_pipeline_info.build()],
                    None,
                )
                .expect("Unable to create graphics pipeline")
        };

        graphics_pipelines[0]
    }

    fn create_compute_shader_modules(
        device: &ash::Device,
        compute_shader_path: &str,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) -> Result<
        (
            Vec<vk::PipelineShaderStageCreateInfo>,
            shader::Reflection,
            vk::PipelineLayout,
            Vec<vk::DescriptorSetLayout>,
        ),
        shaderc::Error,
    > {
        let compute_spv_file = shader::compile_glsl_shader(compute_shader_path)?;
        let compute_spv_file = compute_spv_file.as_binary_u8();

        let reflection = shader::Reflection::new(&[compute_spv_file]);

        let (pipeline_layout, descriptor_set_layouts, _) = shader::create_layouts_from_reflection(
            device,
            &reflection,
            bindless_descriptor_set_layout,
        );

        let compute_spv_file = Cursor::new(compute_spv_file);

        let compute_shader_module = shader::create_shader_module(compute_spv_file, device);

        let shader_entry_name = CStr::from_bytes_with_nul(b"main\0").unwrap();
        let shader_stage_create_infos = vec![vk::PipelineShaderStageCreateInfo {
            module: compute_shader_module,
            p_name: shader_entry_name.as_ptr(),
            stage: vk::ShaderStageFlags::COMPUTE,
            ..Default::default()
        }];

        Ok((
            shader_stage_create_infos,
            reflection,
            pipeline_layout,
            descriptor_set_layouts,
        ))
    }

    fn create_compute_pipeline(
        device: &ash::Device,
        shader_stage_create_infos: Vec<vk::PipelineShaderStageCreateInfo>,
        pipeline_layout: vk::PipelineLayout,
    ) -> vk::Pipeline {
        let create_info = vk::ComputePipelineCreateInfo::builder()
            .stage(shader_stage_create_infos[0])
            .layout(pipeline_layout)
            .build();

        let compute_pipelines = unsafe {
            device
                .create_compute_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .expect("Unable to create compute pipeline")
        };

        compute_pipelines[0]
    }

    fn create_raytracing_shader_modules(
        device: &ash::Device,
        raygen_shader_path: &str,
        miss_shader_path: &str,
        closest_hit_shader_path: &str,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) -> Result<
        (
            Vec<vk::PipelineShaderStageCreateInfo>,
            shader::Reflection,
            vk::PipelineLayout,
            Vec<vk::DescriptorSetLayout>,
        ),
        shaderc::Error,
    > {
        let raygen_spv_file = shader::compile_glsl_shader(raygen_shader_path)?;
        let miss_spv_file = shader::compile_glsl_shader(miss_shader_path)?;
        let closest_hit_spv_file = shader::compile_glsl_shader(closest_hit_shader_path)?;

        let raygen_spv_file = raygen_spv_file.as_binary_u8();
        let miss_spv_file = miss_spv_file.as_binary_u8();
        let closest_hit_spv_file = closest_hit_spv_file.as_binary_u8();

        let reflection =
            shader::Reflection::new(&[raygen_spv_file, miss_spv_file, closest_hit_spv_file]);
        let (pipeline_layout, descriptor_set_layouts, _) = shader::create_layouts_from_reflection(
            &device,
            &reflection,
            bindless_descriptor_set_layout,
        );

        let raygen_spv_file = Cursor::new(raygen_spv_file);
        let miss_spv_file = Cursor::new(miss_spv_file);
        let closest_hit_spv_file = Cursor::new(closest_hit_spv_file);

        let raygen_shader_module = crate::shader::create_shader_module(raygen_spv_file, &device);
        let miss_shader_module = crate::shader::create_shader_module(miss_spv_file, &device);
        let closest_hit_shader_module =
            crate::shader::create_shader_module(closest_hit_spv_file, &device);

        let shader_entry_name = CStr::from_bytes_with_nul(b"main\0").unwrap();
        let shader_stage_create_infos = vec![
            vk::PipelineShaderStageCreateInfo {
                module: raygen_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::RAYGEN_KHR,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: miss_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::MISS_KHR,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: closest_hit_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::CLOSEST_HIT_KHR,
                ..Default::default()
            },
        ];

        Ok((
            shader_stage_create_infos,
            reflection,
            pipeline_layout,
            descriptor_set_layouts,
        ))
    }

    fn create_raytracing_pipeline(
        device: &Device,
        shader_stage_create_infos: Vec<vk::PipelineShaderStageCreateInfo>,
        pipeline_layout: vk::PipelineLayout,
    ) -> vk::Pipeline {
        let shader_group_create_infos = [
            ash::vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(ash::vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(0) // Todo: not hardcode like this
                .closest_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .any_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .intersection_shader(ash::vk::SHADER_UNUSED_KHR)
                .build(),
            ash::vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(ash::vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(1) // Todo: not hardcode like this
                .closest_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .any_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .intersection_shader(ash::vk::SHADER_UNUSED_KHR)
                .build(),
            ash::vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(ash::vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
                .general_shader(ash::vk::SHADER_UNUSED_KHR) // Todo: not hardcode like this
                .closest_hit_shader(2)
                .any_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .intersection_shader(ash::vk::SHADER_UNUSED_KHR)
                .build(),
        ];

        let pipeline_create_info = vk::RayTracingPipelineCreateInfoKHR::builder()
            .max_pipeline_ray_recursion_depth(1)
            .layout(pipeline_layout)
            .stages(&shader_stage_create_infos)
            .groups(&shader_group_create_infos)
            .build();

        let pipeline = unsafe {
            device
                .raytracing_pipeline_ext
                .create_ray_tracing_pipelines(
                    vk::DeferredOperationKHR::null(),
                    vk::PipelineCache::null(),
                    &[pipeline_create_info],
                    None,
                )
                .expect("Failed to create raytracing pipeline")[0]
        };

        pipeline
    }

    fn create_raytracing_sbt(device: &Device, pipeline: vk::Pipeline) -> RayTracingSbt {
        let handle_size = device.rt_pipeline_properties.shader_group_handle_size as usize;
        let group_count = 3; // alignment? note that the size corresponds to shader_group_create_infos
        let sbt_size = group_count * handle_size;

        let shader_handle_storage = unsafe {
            device
                .raytracing_pipeline_ext
                .get_ray_tracing_shader_group_handles(
                    pipeline,
                    0,
                    group_count as u32,
                    sbt_size as usize,
                )
                .expect("Failed to get raytracing shader group handles")
        };

        let raygen_sbt_buffer = Buffer::new(
            device,
            Some(&shader_handle_storage[0..handle_size]),
            handle_size as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR,
            gpu_allocator::MemoryLocation::GpuOnly,
        );

        let miss_sbt_buffer = Buffer::new(
            device,
            Some(&shader_handle_storage[handle_size..handle_size * 2]),
            handle_size as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR,
            gpu_allocator::MemoryLocation::GpuOnly,
        );

        let hit_sbt_buffer = Buffer::new(
            device,
            Some(&shader_handle_storage[handle_size * 2..handle_size * 3]),
            handle_size as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR,
            gpu_allocator::MemoryLocation::GpuOnly,
        );

        RayTracingSbt {
            raygen_sbt: vk::StridedDeviceAddressRegionKHR {
                device_address: raygen_sbt_buffer.get_device_address(device),
                stride: device.rt_pipeline_properties.shader_group_handle_size as u64,
                size: device.rt_pipeline_properties.shader_group_handle_size as u64,
            },
            miss_sbt: vk::StridedDeviceAddressRegionKHR {
                device_address: miss_sbt_buffer.get_device_address(device),
                stride: device.rt_pipeline_properties.shader_group_handle_size as u64,
                size: device.rt_pipeline_properties.shader_group_handle_size as u64,
            },
            hit_sbt: vk::StridedDeviceAddressRegionKHR {
                device_address: hit_sbt_buffer.get_device_address(device),
                stride: device.rt_pipeline_properties.shader_group_handle_size as u64,
                size: device.rt_pipeline_properties.shader_group_handle_size as u64,
            },
            callable_sbt: vk::StridedDeviceAddressRegionKHR {
                device_address: Default::default(),
                stride: 0,
                size: 0,
            },
            raygen_sbt_buffer,
            miss_sbt_buffer,
            hit_sbt_buffer,
        }
    }
}

impl PipelineDesc {
    pub fn builder() -> PipelineDescBuilder {
        PipelineDescBuilder::new()
    }
}

impl PipelineDescBuilder {
    pub fn new() -> Self {
        Self {
            desc: PipelineDesc {
                vertex_path: None,
                fragment_path: None,
                compute_path: None,
                raygen_path: None,
                miss_path: None,
                hit_path: None,
                vertex_input_binding_descriptions: Vec::new(),
                vertex_input_attribute_descriptions: Vec::new(),
                color_attachment_formats: Vec::new(),
                depth_stencil_attachment_format: vk::Format::UNDEFINED,
            },
        }
    }

    pub fn vertex_path(mut self, path: &'static str) -> Self {
        self.desc.vertex_path = Some(path);
        self
    }

    pub fn fragment_path(mut self, path: &'static str) -> Self {
        self.desc.fragment_path = Some(path);
        self
    }

    pub fn compute_path(mut self, path: &'static str) -> Self {
        self.desc.compute_path = Some(path);
        self
    }

    pub fn raygen_path(mut self, path: &'static str) -> Self {
        self.desc.raygen_path = Some(path);
        self
    }

    pub fn miss_path(mut self, path: &'static str) -> Self {
        self.desc.miss_path = Some(path);
        self
    }

    pub fn hit_path(mut self, path: &'static str) -> Self {
        self.desc.hit_path = Some(path);
        self
    }

    pub fn vertex_input_binding_descriptions(
        mut self,
        descriptions: Vec<vk::VertexInputBindingDescription>,
    ) -> Self {
        self.desc.vertex_input_binding_descriptions = descriptions;
        self
    }

    pub fn vertex_input_attribute_descriptions(
        mut self,
        descriptions: Vec<vk::VertexInputAttributeDescription>,
    ) -> Self {
        self.desc.vertex_input_attribute_descriptions = descriptions;
        self
    }

    pub fn default_primitive_vertex_bindings(mut self) -> Self {
        self.desc.vertex_input_binding_descriptions =
            crate::Primitive::get_vertex_input_binding_descriptions();
        self
    }

    pub fn default_primitive_vertex_attributes(mut self) -> Self {
        self.desc.vertex_input_attribute_descriptions =
            crate::Primitive::get_vertex_input_attribute_descriptions();
        self
    }

    pub fn color_attachment_formats(mut self, formats: Vec<vk::Format>) -> Self {
        self.desc.color_attachment_formats = formats;
        self
    }

    pub fn depth_stencil_attachment_format(mut self, format: vk::Format) -> Self {
        self.desc.depth_stencil_attachment_format = format;
        self
    }

    pub fn build(self) -> PipelineDesc {
        self.desc
    }
}
