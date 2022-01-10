use ash::util::*;
use ash::vk;
use std::io::Cursor;
use std::fs;

use utopian;
use shaderc;
use rspirv_reflect;

struct Application {
    base: utopian::VulkanBase,
    renderpass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    pipeline: utopian::Pipeline,
    primitive: utopian::Primitive,
}

impl Application {
    fn new() -> Application {
        let base = utopian::VulkanBase::new(1200, 800);

        let renderpass = Application::create_renderpass(&base);
        let framebuffers = Application::create_framebuffers(&base, renderpass);

        let indices = vec![0u32, 1, 2];

        let vertices = vec![
            utopian::Vertex {
                pos: [-1.0, 1.0, 0.0, 1.0],
                color: [0.0, 1.0, 0.0, 1.0],
            },
            utopian::Vertex {
                pos: [1.0, 1.0, 0.0, 1.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
            utopian::Vertex {
                pos: [0.0, -1.0, 0.0, 1.0],
                color: [1.0, 0.0, 0.0, 1.0],
            },
        ];

        let primitive = utopian::Primitive::new(
            &base.device,
            base.device_memory_properties,
            indices,
            vertices,
        );

        // Todo: understand Cursor
        let vertex_spv_file = Application::compile_glsl_shader("prototype/shaders/triangle/triangle.vert");
        let vertex_spv_file = Cursor::new(vertex_spv_file.as_binary_u8());

        let fragment_spv_file = Application::compile_glsl_shader("prototype/shaders/triangle/triangle.frag");
        let fragment_spv_file = Cursor::new(fragment_spv_file.as_binary_u8());

        let vertex_shader_module = Application::create_shader_module(vertex_spv_file, &base.device);
        let fragment_shader_module =
            Application::create_shader_module(fragment_spv_file, &base.device);

        let pipeline_layout = Application::create_pipeline_layout(&base.device);

        let pipeline = utopian::Pipeline::new(
            &base.device,
            vertex_shader_module,
            fragment_shader_module,
            renderpass,
            pipeline_layout,
            base.surface_resolution,
        );

        Application {
            base,
            renderpass,
            framebuffers,
            pipeline,
            primitive,
        }
    }

    fn compile_glsl_shader(path: &str) -> shaderc::CompilationArtifact {
        let source = &fs::read_to_string(path).expect("Error reading shader file")[..];

        let shader_kind = if path.ends_with(".vert") {
            shaderc::ShaderKind::Vertex
        }
        else if path.ends_with(".frag") {
            shaderc::ShaderKind::Fragment
        }
        else {
            panic!("Unsupported shader extension");
        };

        let mut compiler = shaderc::Compiler::new().unwrap();
        let mut options = shaderc::CompileOptions::new().unwrap();
        options.add_macro_definition("EP", Some("main"));
        let binary_result = compiler.compile_into_spirv(
            source, shader_kind,
            "shader.glsl", "main", Some(&options)).unwrap();

        assert_eq!(Some(&0x07230203), binary_result.as_binary().first());

        let text_result = compiler.compile_into_spirv_assembly(
            source, shader_kind,
            "shader.glsl", "main", Some(&options)).unwrap();

        assert!(text_result.as_text().starts_with("; SPIR-V\n"));

        println!("{}", text_result.as_text());

        let reflection = rspirv_reflect::Reflection::new_from_spirv(binary_result.as_binary_u8())
            .expect("Shader reflection failed!");

        // Test reflection
        let descriptor_sets = reflection.get_descriptor_sets();
        //let push_constants = reflection.get_push_constant_range().unwrap().unwrap();

        println!("{:#?}", descriptor_sets);
        // println!("{:#?}", push_constants.size);
        // println!("{:#?}", push_constants.offset);

        binary_result
    }

    fn create_renderpass(base: &utopian::vulkan_base::VulkanBase) -> vk::RenderPass {
        let renderpass_attachments = [
            vk::AttachmentDescription {
                format: base.surface_format.format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                ..Default::default()
            },
            // vk::AttachmentDescription {
            //     format: vk::Format::D16_UNORM,
            //     samples: vk::SampleCountFlags::TYPE_1,
            //     load_op: vk::AttachmentLoadOp::CLEAR,
            //     initial_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            //     final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            //     ..Default::default()
            // },
        ];
        let color_attachment_refs = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        // let depth_attachment_ref = vk::AttachmentReference {
        //     attachment: 1,
        //     layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        // };
        let dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ..Default::default()
        }];

        let subpass = vk::SubpassDescription::builder()
            .color_attachments(&color_attachment_refs)
            //.depth_stencil_attachment(&depth_attachment_ref)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        let renderpass_create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&renderpass_attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);

        let renderpass = unsafe {
            base.device
                .create_render_pass(&renderpass_create_info, None)
                .expect("Failed to create renderpass")
        };

        renderpass
    }

    fn create_framebuffers(base: &utopian::vulkan_base::VulkanBase, renderpass: vk::RenderPass) -> Vec<vk::Framebuffer> {
        let framebuffers: Vec<vk::Framebuffer> = base
            .present_image_views
            .iter()
            .map(|&present_image_view| {
                //let framebuffer_attachments = [present_image_view, base.depth_image_view];
                let framebuffer_attachments = [present_image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(renderpass)
                    .attachments(&framebuffer_attachments)
                    .width(base.surface_resolution.width)
                    .height(base.surface_resolution.height)
                    .layers(1);

                unsafe {
                    base.device
                        .create_framebuffer(&frame_buffer_create_info, None)
                        .unwrap()
                }
            })
            .collect();

        framebuffers
    }

    fn create_shader_module(mut spv_file: Cursor<&[u8]>, device: &ash::Device) -> vk::ShaderModule {
        let shader_code = read_spv(&mut spv_file).expect("Failed to read shader spv file");
        let shader_info = vk::ShaderModuleCreateInfo::builder().code(&shader_code);

        let shader_module = unsafe {
            device
                .create_shader_module(&shader_info, None)
                .expect("Error creating shader module")
        };

        shader_module
    }

    fn create_pipeline_layout(device: &ash::Device) -> vk::PipelineLayout {
        let layout_create_info = vk::PipelineLayoutCreateInfo::default();

        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&layout_create_info, None)
                .expect("Error creating pipeline layout")
        };

        pipeline_layout
    }

    fn record_commands<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        wait_fence: vk::Fence,
        render_commands: F,
    ) {
        unsafe {
            device
                .wait_for_fences(&[wait_fence], true, std::u64::MAX)
                .expect("Wait for fence failed.");

            device
                .reset_fences(&[wait_fence])
                .expect("Reset fences failed.");

            device
                .reset_command_buffer(
                    command_buffer,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .expect("Reset command buffer failed.");

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .expect("Begin command buffer failed.");

            render_commands(&device, command_buffer);

            device
                .end_command_buffer(command_buffer)
                .expect("End commandbuffer failed.");
        }
    }

    fn run(&self) {
        self.base.run(|| unsafe {
            let present_index = self.base.prepare_frame();

            Application::record_commands(
                &self.base.device,
                self.base.draw_command_buffer,
                self.base.draw_commands_reuse_fence,
                |device, command_buffer| {
                    let clear_values = [
                        vk::ClearValue {
                            color: vk::ClearColorValue {
                                float32: [0.5, 0.5, 0.5, 0.0],
                            },
                        },
                        vk::ClearValue {
                            depth_stencil: vk::ClearDepthStencilValue {
                                depth: 1.0,
                                stencil: 0,
                            },
                        },
                    ];

                    let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                        .render_pass(self.renderpass)
                        .framebuffer(self.framebuffers[present_index as usize])
                        .render_area(vk::Rect2D {
                            offset: vk::Offset2D { x: 0, y: 0 },
                            extent: self.base.surface_resolution,
                        })
                        .clear_values(&clear_values);

                    device.cmd_begin_render_pass(
                        command_buffer,
                        &render_pass_begin_info,
                        vk::SubpassContents::INLINE,
                    );

                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline.handle,
                    );

                    let viewports = [vk::Viewport {
                        x: 0.0,
                        y: 0.0,
                        width: self.base.surface_resolution.width as f32,
                        height: self.base.surface_resolution.height as f32,
                        min_depth: 0.0,
                        max_depth: 1.0,
                    }];

                    let scissors = [vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.base.surface_resolution,
                    }];

                    device.cmd_set_viewport(command_buffer, 0, &viewports);
                    device.cmd_set_scissor(command_buffer, 0, &scissors);
                    device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[self.primitive.vertex_buffer.buffer],
                        &[0],
                    );
                    device.cmd_bind_index_buffer(
                        command_buffer,
                        self.primitive.index_buffer.buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    device.cmd_draw_indexed(
                        command_buffer,
                        self.primitive.indices.len() as u32,
                        1,
                        0,
                        0,
                        1,
                    );

                    device.cmd_end_render_pass(command_buffer);
                },
            );

            self.base.submit_commands();
            self.base.present_frame(present_index);
        });
    }
}

fn main() {
    let app = Application::new();

    app.run();

    println!("End!");
}

