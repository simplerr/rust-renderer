use ash::vk;

use utopian;

struct Application {
    base: utopian::VulkanBase,
    renderpass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    pipeline: utopian::Pipeline,
    primitive: utopian::Primitive,
    descriptor_set: utopian::DescriptorSet,      // testing
    descriptor_set_frag: utopian::DescriptorSet, // testing
    binding1: utopian::shader::Binding,          // testing
    binding2: utopian::shader::Binding,          // testing
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

        let pipeline = utopian::Pipeline::new(
            &base.device,
            "prototype/shaders/triangle/triangle.vert",
            "prototype/shaders/triangle/triangle.frag",
            renderpass,
            base.surface_resolution,
        );

        let binding1 = pipeline.reflection.get_binding("test1");
        let binding2 = pipeline.reflection.get_binding("test_frag");

        let descriptor_set = utopian::DescriptorSet::new(
            &base.device,
            pipeline.descriptor_set_layouts[binding1.set as usize],
            pipeline.reflection.get_set_mappings(binding1.set),
        );

        let descriptor_set_frag = utopian::DescriptorSet::new(
            &base.device,
            pipeline.descriptor_set_layouts[binding2.set as usize],
            pipeline.reflection.get_set_mappings(binding2.set),
        );

        let uniform_data = [1.0f32, 0.0, 0.0, 1.0];
        let uniform_buffer = utopian::Buffer::new(
            &base.device,
            base.device_memory_properties,
            &uniform_data,
            std::mem::size_of_val(&uniform_data) as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        );

        let uniform_data_frag = [0.0f32, 1.0, 0.0, 1.0];
        let uniform_buffer_frag = utopian::Buffer::new(
            &base.device,
            base.device_memory_properties,
            &uniform_data_frag,
            std::mem::size_of_val(&uniform_data) as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        );

        descriptor_set.write_uniform_buffer(&base.device, "test1".to_string(), &uniform_buffer);
        descriptor_set.write_uniform_buffer(
            &base.device,
            "test2".to_string(),
            &uniform_buffer_frag,
        );
        descriptor_set_frag.write_uniform_buffer(
            &base.device,
            "test_frag".to_string(),
            &uniform_buffer_frag,
        );

        Application {
            base,
            renderpass,
            framebuffers,
            pipeline,
            primitive,
            descriptor_set,
            descriptor_set_frag,
            binding1,
            binding2,
        }
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

    fn create_framebuffers(
        base: &utopian::vulkan_base::VulkanBase,
        renderpass: vk::RenderPass,
    ) -> Vec<vk::Framebuffer> {
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

                    let push_data = [1.0f32, 0.5, 0.2, 1.0];

                    device.cmd_push_constants(
                        command_buffer,
                        self.pipeline.pipeline_layout,
                        vk::ShaderStageFlags::ALL,
                        0,
                        std::slice::from_raw_parts(
                            push_data.as_ptr() as *const u8,
                            std::mem::size_of_val(&push_data),
                        ),
                    );

                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline.pipeline_layout,
                        self.binding1.set,
                        &[self.descriptor_set.handle],
                        &[],
                    );

                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline.pipeline_layout,
                        self.binding2.set,
                        &[self.descriptor_set_frag.handle],
                        &[],
                    );

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
