use ash::vk;
use std::ffi::CStr;
use std::io::Cursor;
use std::mem;

use crate::offset_of;
use crate::*;

pub struct PipelineDesc {
    pub vertex_path: &'static str,
    pub fragment_path: &'static str,
}

pub struct Pipeline {
    pub handle: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    pub reflection: shader::Reflection,
    pub pipeline_desc: PipelineDesc,
}

impl Pipeline {
    pub fn new(
        device: &ash::Device,
        pipeline_desc: PipelineDesc,
        renderpass: vk::RenderPass,
        surface_resolution: vk::Extent2D,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) -> Pipeline {
        let shader_modules_result = Pipeline::create_shader_modules(
            device,
            pipeline_desc.vertex_path,
            pipeline_desc.fragment_path,
            bindless_descriptor_set_layout,
        );

        let (shader_stage_create_infos, reflection, pipeline_layout, descriptor_set_layouts) =
            shader_modules_result.expect("Failed to create shader modules");

        let graphic_pipeline = Pipeline::create_pipeline(
            device,
            shader_stage_create_infos,
            renderpass,
            pipeline_layout,
            surface_resolution,
        );

        Pipeline {
            handle: graphic_pipeline,
            pipeline_layout,
            descriptor_set_layouts,
            reflection,
            pipeline_desc,
        }
    }

    fn create_shader_modules(
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

    fn create_pipeline(
        device: &ash::Device,
        shader_stage_create_infos: Vec<vk::PipelineShaderStageCreateInfo>,
        renderpass: vk::RenderPass,
        pipeline_layout: vk::PipelineLayout,
        surface_resolution: vk::Extent2D,
    ) -> vk::Pipeline {
        let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let vertex_input_attribute_descriptions = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, normal) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, uv) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 3,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, color) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 4,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, tangent) as u32,
            },
        ];

        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);
        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            ..Default::default()
        };
        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: surface_resolution.width as f32,
            height: surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: surface_resolution,
        }];
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissors(&scissors)
            .viewports(&viewports);

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
        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
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
        }];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);

        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_state);

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
            .render_pass(renderpass);

        let graphics_pipelines = unsafe {
            device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[graphic_pipeline_info.build()],
                    None,
                )
                .expect("Unable to create graphics pipeline")
        };

        let graphic_pipeline = graphics_pipelines[0];

        graphic_pipeline
    }

    pub fn recreate_pipeline(
        &mut self,
        device: &Device,
        renderpass: vk::RenderPass,
        surface_resolution: vk::Extent2D,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) {
        // Todo: cleanup old resources

        let shader_modules_result = Pipeline::create_shader_modules(
            &device.handle,
            self.pipeline_desc.vertex_path,
            self.pipeline_desc.fragment_path,
            bindless_descriptor_set_layout,
        );

        match shader_modules_result {
            Ok((
                shader_stage_create_infos,
                _reflection,
                pipeline_layout,
                _descriptor_set_layouts,
            )) => {
                let graphic_pipeline = Pipeline::create_pipeline(
                    &device.handle,
                    shader_stage_create_infos,
                    renderpass,
                    pipeline_layout,
                    surface_resolution,
                );

                println!("{} and {} was successfully recompiled", self.pipeline_desc.vertex_path, self.pipeline_desc.fragment_path);

                self.handle = graphic_pipeline
            }
            Err(error) => {
                println!("Failed to recreate rasterization pipeline: {:#?}", error);
            }
        }
    }
}
