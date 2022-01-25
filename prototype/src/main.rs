use ash::vk;
use glam::Vec3;

use utopian;

#[derive(Clone, Debug, Copy)]
struct CameraUniformData {
    view_mat: glam::Mat4,
    projection_mat: glam::Mat4,
    eye_pos: glam::Vec3,
}

struct PushConstants {
    world: glam::Mat4,
    color: glam::Vec4,
    diffuse_tex_id: u32,
    pad: glam::Vec3,
}

struct Application {
    base: utopian::VulkanBase,
    renderpass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    pipeline: utopian::Pipeline,
    model: utopian::Model,
    descriptor_set_camera: utopian::DescriptorSet,      // testing
    descriptor_set_bindless: utopian::DescriptorSet, // testing
    camera_binding: utopian::shader::Binding,          // testing
    bindless_binding: utopian::shader::Binding,          // testing
    camera_data: CameraUniformData,
    camera_ubo: utopian::Buffer,
    camera: utopian::Camera,
}

impl Application {
    fn new() -> Application {
        let (width, height) = (2000, 1100);
        let base = utopian::VulkanBase::new(width, height);

        let renderpass = Application::create_renderpass(&base);
        let framebuffers = Application::create_framebuffers(&base, renderpass);

        let model =
            //utopian::gltf_loader::load_gltf(&base.device, "prototype/data/models/sphere.gltf");
            //utopian::gltf_loader::load_gltf(&base.device, "prototype/data/models/Sponza/glTF/Sponza.gltf");
            utopian::gltf_loader::load_gltf(&base.device, "prototype/data/models/FlightHelmet/glTF/FlightHelmet.gltf");
        //utopian::ModelLoader::load_cube(&base.device);

        let camera = utopian::Camera::new(
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.0, 0.0, 0.0),
            60.0,
            width as f32 / height as f32,
            0.01,
            20000.0,
            0.002,
        );

        let camera_data = CameraUniformData {
            view_mat: camera.get_view(),
            projection_mat: camera.get_projection(),
            eye_pos: camera.get_position(),
        };

        let slice = unsafe { std::slice::from_raw_parts(&camera_data, 1) };

        let camera_uniform_buffer = utopian::Buffer::new(
            &base.device,
            &slice,
            std::mem::size_of_val(&camera_data) as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        );

        let bindless_descriptor_set = Application::create_bindless_descriptor_set_layout(&base.device);

        let pipeline = utopian::Pipeline::new(
            &base.device.handle,
            "prototype/shaders/triangle/triangle.vert",
            "prototype/shaders/triangle/triangle.frag",
            renderpass,
            base.surface_resolution,
            Some(bindless_descriptor_set),
        );

        let camera_binding = pipeline.reflection.get_binding("camera");
        let bindless_binding = pipeline.reflection.get_binding("samplerColor");

        let descriptor_set_camera = utopian::DescriptorSet::new(
            &base.device,
            pipeline.descriptor_set_layouts[camera_binding.set as usize],
            pipeline.reflection.get_set_mappings(camera_binding.set),
        );

        let descriptor_set_bindless = utopian::DescriptorSet::new(
            &base.device,
            pipeline.descriptor_set_layouts[bindless_binding.set as usize],
            pipeline.reflection.get_set_mappings(bindless_binding.set),
        );

        let uniform_data = [1.0f32, 0.0, 0.0, 1.0];
        let uniform_buffer = utopian::Buffer::new(
            &base.device,
            &uniform_data,
            std::mem::size_of_val(&uniform_data) as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        );

        let uniform_data_frag = [0.0f32, 1.0, 0.0, 1.0];
        let uniform_buffer_frag = utopian::Buffer::new(
            &base.device,
            &uniform_data_frag,
            std::mem::size_of_val(&uniform_data) as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        );

        let texture = utopian::Texture::load(&base.device, "prototype/data/rust.png");

        descriptor_set_camera.write_uniform_buffer(
            &base.device,
            "camera".to_string(),
            &camera_uniform_buffer,
        );

        //descriptor_set.write_combined_image(&base.device, "samplerColor".to_string(), &texture);
        descriptor_set_bindless.write_combined_image(
            &base.device,
            "samplerColor".to_string(),
            &model.textures[3],
        );

        Application {
            base,
            renderpass,
            framebuffers,
            pipeline,
            model,
            descriptor_set_camera,
            descriptor_set_bindless,
            camera_binding,
            bindless_binding,
            camera_data,
            camera_ubo: camera_uniform_buffer,
            camera,
        }
    }

    fn create_bindless_descriptor_set_layout(device: &utopian::Device) -> vk::DescriptorSetLayout {
        let descriptor_set_layout_binding =
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::ALL)
                .build();

        let mut binding_flags: Vec<vk::DescriptorBindingFlags> =
            vec![
                vk::DescriptorBindingFlags::PARTIALLY_BOUND
                //    | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT;
            ];

        let mut binding_flags_create_info =
            vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
                .binding_flags(&binding_flags);

        let descriptor_sets_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&[descriptor_set_layout_binding])
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
            .push_next(&mut binding_flags_create_info)
            .build();

        let descriptor_set_layout = unsafe {
            device.handle
                .create_descriptor_set_layout(&descriptor_sets_layout_info, None)
                .expect("Error creating descriptor set layout")
        };

        descriptor_set_layout
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
            vk::AttachmentDescription {
                format: vk::Format::D32_SFLOAT,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                initial_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
        ];
        let color_attachment_refs = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };
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
            .depth_stencil_attachment(&depth_attachment_ref)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        let renderpass_create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&renderpass_attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);

        let renderpass = unsafe {
            base.device
                .handle
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
                let framebuffer_attachments = [present_image_view, base.depth_image.image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(renderpass)
                    .attachments(&framebuffer_attachments)
                    .width(base.surface_resolution.width)
                    .height(base.surface_resolution.height)
                    .layers(1);

                unsafe {
                    base.device
                        .handle
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

    fn run(&mut self) {
        self.base.run(|input| unsafe {
            let present_index = self.base.prepare_frame();

            self.camera.update(&input);

            self.camera_data.view_mat = self.camera.get_view();
            self.camera_data.projection_mat = self.camera.get_projection();
            self.camera_data.eye_pos = self.camera.get_position();

            self.camera_ubo.update_memory(
                &self.base.device,
                std::slice::from_raw_parts(&self.camera_data, 1),
            );

            Application::record_commands(
                &self.base.device.handle,
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
                        y: self.base.surface_resolution.height as f32,
                        width: self.base.surface_resolution.width as f32,
                        height: -(self.base.surface_resolution.height as f32),
                        min_depth: 0.0,
                        max_depth: 1.0,
                    }];

                    let scissors = [vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.base.surface_resolution,
                    }];

                    device.cmd_set_viewport(command_buffer, 0, &viewports);
                    device.cmd_set_scissor(command_buffer, 0, &scissors);

                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline.pipeline_layout,
                        self.camera_binding.set,
                        &[self.descriptor_set_camera.handle],
                        &[],
                    );

                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline.pipeline_layout,
                        self.bindless_binding.set,
                        &[self.descriptor_set_bindless.handle],
                        &[],
                    );

                    for (i, primitive) in self.model.primitives.iter().enumerate() {
                        let model_world =
                            glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, 0.0));
                        let push_data = PushConstants {
                            world: model_world * self.model.transforms[i],
                            color: glam::Vec4::new(1.0, 0.5, 0.2, 1.0),
                            diffuse_tex_id: 2,
                            pad: glam::Vec3::new(0.0, 0.0, 0.0),
                        };

                        device.cmd_push_constants(
                            command_buffer,
                            self.pipeline.pipeline_layout,
                            vk::ShaderStageFlags::ALL,
                            0,
                            std::slice::from_raw_parts(
                                &push_data as *const _ as *const u8,
                                std::mem::size_of_val(&push_data),
                            ),
                        );

                        device.cmd_bind_vertex_buffers(
                            command_buffer,
                            0,
                            &[primitive.vertex_buffer.buffer],
                            &[0],
                        );
                        device.cmd_bind_index_buffer(
                            command_buffer,
                            primitive.index_buffer.buffer,
                            0,
                            vk::IndexType::UINT32,
                        );
                        device.cmd_draw_indexed(
                            command_buffer,
                            primitive.indices.len() as u32,
                            1,
                            0,
                            0,
                            1,
                        );
                    }

                    device.cmd_end_render_pass(command_buffer);
                },
            );

            self.base.submit_commands();
            self.base.present_frame(present_index);
        });
    }
}

fn main() {
    let mut app = Application::new();

    app.run();

    println!("End!");
}
