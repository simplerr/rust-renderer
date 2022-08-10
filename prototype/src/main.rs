use ash::vk;
use glam::{Quat, Vec3};
use gpu_allocator::vulkan::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Clone, Debug, Copy)]
struct CameraUniformData {
    view: glam::Mat4,
    projection: glam::Mat4,
    inverse_view: glam::Mat4,
    inverse_projection: glam::Mat4,
    eye_pos: glam::Vec3,
    samples_per_frame: u32,
    total_samples: u32,
    num_bounces: u32,
}

#[allow(dead_code)]
struct PushConstants {
    world: glam::Mat4,
    color: glam::Vec4,
    mesh_index: u32,
    pad: [u32; 3],
}

struct FpsTimer {
    fps_period_start_time: Instant,
    fps: u32,
    elapsed_frames: u32,
}

struct Application {
    base: utopian::VulkanBase,
    renderpass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    pipeline: utopian::Pipeline,
    descriptor_set_camera: utopian::DescriptorSet,
    camera_binding: utopian::shader::Binding,
    camera_data: CameraUniformData,
    camera_ubo: utopian::Buffer,
    camera: utopian::Camera,
    renderer: utopian::Renderer,
    raytracing: utopian::Raytracing,
    egui_integration:
        egui_winit_ash_integration::Integration<Arc<Mutex<gpu_allocator::vulkan::Allocator>>>,
    fps_timer: FpsTimer,
    raytracing_enabled: bool,

    // For automatic shader recompilation
    // Todo: generalize and move from here
    _directory_watcher: notify::ReadDirectoryChangesWatcher,
    watcher_rx: mpsc::Receiver<notify::DebouncedEvent>,
}

impl FpsTimer {
    fn calculate(&mut self) -> u32 {
        self.elapsed_frames += 1;
        let elapsed = self.fps_period_start_time.elapsed().as_millis() as u32;
        if elapsed > 1000 {
            self.fps = self.elapsed_frames;
            self.fps_period_start_time = Instant::now();
            self.elapsed_frames = 0;
        }

        self.fps
    }
}

impl Application {
    fn new() -> Application {
        let (width, height) = (2000, 1100);
        let base = utopian::VulkanBase::new(width, height);

        let renderer = utopian::Renderer::new(&base.device);

        let renderpass = Application::create_renderpass(&base);
        let framebuffers = Application::create_framebuffers(&base, renderpass);

        let camera = utopian::Camera::new(
            //Vec3::new(0.0, 0.9, 2.0), // Cornell box scene
            Vec3::new(-10.28, 2.10, -0.18), // Sponza scene
            Vec3::new(0.0, 0.5, 0.0),
            60.0,
            width as f32 / height as f32,
            0.01,
            20000.0,
            0.002,
        );

        let camera_data = CameraUniformData {
            view: camera.get_view(),
            projection: camera.get_projection(),
            inverse_view: camera.get_view().inverse(),
            inverse_projection: camera.get_projection().inverse(),
            eye_pos: camera.get_position(),
            samples_per_frame: 1,
            total_samples: 0,
            num_bounces: 5,
        };

        let slice = unsafe { std::slice::from_raw_parts(&camera_data, 1) };

        let camera_uniform_buffer = utopian::Buffer::new(
            &base.device,
            Some(slice),
            std::mem::size_of_val(&camera_data) as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let pipeline = utopian::Pipeline::new(
            &base.device.handle,
            utopian::PipelineDesc {
                vertex_path: "prototype/shaders/pbr/pbr.vert",
                fragment_path: "prototype/shaders/pbr/pbr.frag",
            },
            renderpass,
            base.surface_resolution,
            Some(renderer.bindless_descriptor_set_layout),
        );

        let camera_binding = pipeline.reflection.get_binding("camera");

        let descriptor_set_camera = utopian::DescriptorSet::new(
            &base.device,
            pipeline.descriptor_set_layouts[camera_binding.set as usize],
            pipeline.reflection.get_set_mappings(camera_binding.set),
        );

        descriptor_set_camera.write_uniform_buffer(
            &base.device,
            "camera".to_string(),
            &camera_uniform_buffer,
        );

        // Prepare gpu-allocator's Allocator
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: base.instance.clone(),
            device: base.device.handle.clone(),
            physical_device: base.device.physical_device,
            debug_settings: Default::default(),
            buffer_device_address: false,
        })
        .unwrap();

        let allocator = Arc::new(Mutex::new(allocator));

        let egui_integration = egui_winit_ash_integration::Integration::new(
            width,
            height,
            1.0,
            egui::FontDefinitions::default(),
            egui::Style::default(),
            base.device.handle.clone(),
            allocator,
            base.swapchain_loader.clone(),
            base.swapchain,
            base.surface_format,
        );

        let raytracing = utopian::Raytracing::new(
            &base.device,
            &camera_uniform_buffer,
            base.surface_resolution,
            Some(renderer.bindless_descriptor_set_layout),
        );

        let (watcher_tx, watcher_rx) = mpsc::channel();
        let mut directory_watcher: RecommendedWatcher =
            Watcher::new(watcher_tx, Duration::from_millis(100)).unwrap();
        directory_watcher
            .watch("utopian/shaders/", RecursiveMode::Recursive)
            .unwrap();
        directory_watcher
            .watch("prototype/shaders", RecursiveMode::Recursive)
            .unwrap();

        Application {
            base,
            renderpass,
            framebuffers,
            pipeline,
            descriptor_set_camera,
            camera_binding,
            camera_data,
            camera_ubo: camera_uniform_buffer,
            camera,
            renderer,
            raytracing,
            egui_integration,
            fps_timer: FpsTimer {
                fps_period_start_time: Instant::now(),
                fps: 0,
                elapsed_frames: 0,
            },
            raytracing_enabled: true,
            _directory_watcher: directory_watcher,
            watcher_rx,
        }
    }

    fn create_renderpass(base: &utopian::vulkan_base::VulkanBase) -> vk::RenderPass {
        let renderpass_attachments = [
            vk::AttachmentDescription {
                format: base.surface_format.format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
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

        unsafe {
            base.device
                .handle
                .create_render_pass(&renderpass_create_info, None)
                .expect("Failed to create renderpass")
        }
    }

    fn create_framebuffers(
        base: &utopian::vulkan_base::VulkanBase,
        renderpass: vk::RenderPass,
    ) -> Vec<vk::Framebuffer> {
        let framebuffers: Vec<vk::Framebuffer> = base
            .present_images
            .iter()
            .map(|present_image| {
                let framebuffer_attachments =
                    [present_image.image_view, base.depth_image.image_view];
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

    fn record_commands<F: FnOnce(&utopian::Device, vk::CommandBuffer)>(
        device: &utopian::Device,
        command_buffer: vk::CommandBuffer,
        wait_fence: vk::Fence,
        render_commands: F,
    ) {
        unsafe {
            device
                .handle
                .wait_for_fences(&[wait_fence], true, std::u64::MAX)
                .expect("Wait for fence failed.");

            device
                .handle
                .reset_fences(&[wait_fence])
                .expect("Reset fences failed.");

            device
                .handle
                .reset_command_buffer(
                    command_buffer,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .expect("Reset command buffer failed.");

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            device
                .handle
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .expect("Begin command buffer failed.");

            render_commands(device, command_buffer);

            device
                .handle
                .end_command_buffer(command_buffer)
                .expect("End commandbuffer failed.");
        }
    }

    fn create_scene(&mut self) {
        self.renderer.initialize(&self.base.device);

        // =================
        // Cornell box scene
        // =================

        // let cornell_box = utopian::gltf_loader::load_gltf(
        //     &self.base.device,
        //     "prototype/data/models/CornellBox-Original.gltf",
        // );
        //
        // let flight_helmet = utopian::gltf_loader::load_gltf(
        //     &self.base.device,
        //     "prototype/data/models/FlightHelmet/glTF/FlightHelmet.gltf",
        // );
        //
        // self.renderer.add_model(
        //     &self.base.device,
        //     cornell_box,
        //     glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, 0.0)),
        // );
        //
        // let mut light = utopian::ModelLoader::load_cube(&self.base.device);
        // light.meshes[0].material.material_type = utopian::gltf_loader::MaterialType::DiffuseLight;
        //
        // self.renderer.add_model(
        //     &self.base.device,
        //     light,
        //     glam::Mat4::from_scale_rotation_translation(
        //         Vec3::new(0.50, 0.05, 0.35),
        //         Quat::IDENTITY,
        //         glam::Vec3::new(0.0, 1.95, 0.0),
        //     ),
        // );
        //
        // self.renderer.add_model(
        //     &self.base.device,
        //     flight_helmet,
        //     glam::Mat4::from_translation(glam::Vec3::new(-0.33, 0.4, 0.3)),
        // );

        // ============
        // Sponza scene
        // ============

        let sponza = utopian::gltf_loader::load_gltf(
            &self.base.device,
            "prototype/data/models/Sponza/glTF/Sponza.gltf",
        );

        let mut metal_sphere =
            utopian::gltf_loader::load_gltf(&self.base.device, "prototype/data/models/sphere.gltf");
        metal_sphere.meshes[0].material.material_type = utopian::gltf_loader::MaterialType::Metal;
        let mut dielectric_sphere =
            utopian::gltf_loader::load_gltf(&self.base.device, "prototype/data/models/sphere.gltf");
        dielectric_sphere.meshes[0].material.material_type =
            utopian::gltf_loader::MaterialType::Dielectric;
        dielectric_sphere.meshes[0].material.material_property = 1.5;

        self.renderer.add_model(
            &self.base.device,
            sponza,
            glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, 0.0)),
        );

        let size = 0.6;
        self.renderer.add_model(
            &self.base.device,
            metal_sphere,
            glam::Mat4::from_scale_rotation_translation(
                glam::Vec3::new(size, size, size),
                Quat::IDENTITY,
                glam::Vec3::new(-3.0, 0.65, -1.0),
            ),
        );

        self.renderer.add_model(
            &self.base.device,
            dielectric_sphere,
            glam::Mat4::from_scale_rotation_translation(
                glam::Vec3::new(size, size, size),
                Quat::IDENTITY,
                glam::Vec3::new(-3.0, 0.65, 0.7),
            ),
        );

        self.raytracing
            .initialize(&self.base.device, &self.renderer.instances);
    }

    fn update_ui(
        egui_context: &egui::CtxRef,
        camera_pos: &mut Vec3,
        camera_dir: &mut Vec3,
        fps: u32,
        samples_per_frame: &mut u32,
        num_bounces: &mut u32,
    ) {
        egui::Window::new("rust-renderer 0.0.1")
            .resizable(true)
            .scroll(true)
            .show(egui_context, |ui| {
                ui.label(format!("FPS: {}", fps));
                ui.label("Camera position");
                ui.horizontal(|ui| {
                    ui.label("x:");
                    ui.add(egui::widgets::DragValue::new(&mut camera_pos.x));
                    ui.label("y:");
                    ui.add(egui::widgets::DragValue::new(&mut camera_pos.y));
                    ui.label("z:");
                    ui.add(egui::widgets::DragValue::new(&mut camera_pos.z));
                });
                ui.label("Camera direction");
                ui.horizontal(|ui| {
                    ui.label("x:");
                    ui.add(egui::widgets::DragValue::new(&mut camera_dir.x));
                    ui.label("y:");
                    ui.add(egui::widgets::DragValue::new(&mut camera_dir.y));
                    ui.label("z:");
                    ui.add(egui::widgets::DragValue::new(&mut camera_dir.z));
                });
                ui.horizontal(|ui| {
                    ui.label("Samples per frame:");
                    ui.add(egui::widgets::Slider::new(samples_per_frame, 0..=10));
                });
                ui.horizontal(|ui| {
                    ui.label("Num ray bounces:");
                    ui.add(egui::widgets::Slider::new(num_bounces, 0..=150));
                });
            });
    }

    fn run(&mut self) {
        self.base.run(|input, events| unsafe {
            let present_index = self.base.prepare_frame();

            // Update egui input
            // Note: this is not very pretty since we are recreating a Event from
            // an WindowEvent manually. Don't know enough rust to have the `events`
            // input of the correct type.
            for event in events.clone() {
                self.egui_integration
                    .handle_event::<winit::event::Event<winit::event::WindowEvent>>(
                        &winit::event::Event::WindowEvent {
                            window_id: self.base.window.id(),
                            event: event.clone(),
                        },
                    );
            }

            self.egui_integration.begin_frame();

            // Todo: refactor so not everything needs to be passed as argument
            let old_samples_per_frame = self.camera_data.samples_per_frame;
            let old_num_bounces = self.camera_data.num_bounces;
            Application::update_ui(
                &self.egui_integration.context(),
                &mut self.camera.get_position(),
                &mut self.camera.get_forward(),
                self.fps_timer.calculate(),
                &mut self.camera_data.samples_per_frame,
                &mut self.camera_data.num_bounces,
            );

            if self.camera_data.samples_per_frame != old_samples_per_frame
                || self.camera_data.num_bounces != old_num_bounces
            {
                self.camera_data.total_samples = 0;
            }

            if input.key_pressed(winit::event::VirtualKeyCode::Space) {
                self.raytracing_enabled = !self.raytracing_enabled;
            }

            let mut recompile_shaders = false;

            if let Ok(_event) = self.watcher_rx.try_recv() {
                match self.watcher_rx.recv() {
                    Ok(event) => {
                        if let notify::DebouncedEvent::Write(..) = event {
                            recompile_shaders = true
                        }
                    }
                    Err(e) => println!("recv Err {:?}", e),
                }
            }

            if recompile_shaders || input.key_pressed(winit::event::VirtualKeyCode::R) {
                self.camera_data.total_samples = 0;
                self.pipeline.recreate_pipeline(
                    &self.base.device,
                    self.renderpass,
                    self.base.surface_resolution,
                    Some(self.renderer.bindless_descriptor_set_layout),
                );
                self.raytracing.recreate_pipeline(
                    &self.base.device,
                    Some(self.renderer.bindless_descriptor_set_layout),
                );
            }

            if self.camera.update(input) {
                self.camera_data.total_samples = 0;
            }

            self.camera_data.view = self.camera.get_view();
            self.camera_data.projection = self.camera.get_projection();
            self.camera_data.inverse_view = self.camera.get_view().inverse();
            self.camera_data.inverse_projection = self.camera.get_projection().inverse();
            self.camera_data.eye_pos = self.camera.get_position();

            if self.raytracing_enabled {
                self.camera_data.total_samples += self.camera_data.samples_per_frame;
            }

            Application::record_commands(
                &self.base.device,
                self.base.draw_command_buffer,
                self.base.draw_commands_reuse_fence,
                |device, command_buffer| {
                    self.camera_ubo.update_memory(
                        &self.base.device,
                        std::slice::from_raw_parts(&self.camera_data, 1),
                    );

                    if self.raytracing_enabled {
                        self.raytracing.record_commands(
                            device,
                            command_buffer,
                            self.renderer.bindless_descriptor_set,
                            &self.base.present_images[present_index as usize],
                        );
                    } else {
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

                        device.handle.cmd_begin_render_pass(
                            command_buffer,
                            &render_pass_begin_info,
                            vk::SubpassContents::INLINE,
                        );

                        device.handle.cmd_bind_pipeline(
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

                        device
                            .handle
                            .cmd_set_viewport(command_buffer, 0, &viewports);
                        device.handle.cmd_set_scissor(command_buffer, 0, &scissors);

                        device.handle.cmd_bind_descriptor_sets(
                            command_buffer,
                            vk::PipelineBindPoint::GRAPHICS,
                            self.pipeline.pipeline_layout,
                            self.camera_binding.set,
                            &[self.descriptor_set_camera.handle],
                            &[],
                        );

                        device.handle.cmd_bind_descriptor_sets(
                            command_buffer,
                            vk::PipelineBindPoint::GRAPHICS,
                            self.pipeline.pipeline_layout,
                            utopian::BINDLESS_DESCRIPTOR_INDEX,
                            &[self.renderer.bindless_descriptor_set],
                            &[],
                        );

                        for instance in &self.renderer.instances {
                            for (i, mesh) in instance.model.meshes.iter().enumerate() {
                                let push_data = PushConstants {
                                    world: instance.transform * instance.model.transforms[i],
                                    color: glam::Vec4::new(1.0, 0.5, 0.2, 1.0),
                                    mesh_index: mesh.gpu_mesh,
                                    pad: [0; 3],
                                };

                                device.handle.cmd_push_constants(
                                    command_buffer,
                                    self.pipeline.pipeline_layout,
                                    vk::ShaderStageFlags::ALL,
                                    0,
                                    std::slice::from_raw_parts(
                                        &push_data as *const _ as *const u8,
                                        std::mem::size_of_val(&push_data),
                                    ),
                                );

                                device.handle.cmd_bind_vertex_buffers(
                                    command_buffer,
                                    0,
                                    &[mesh.primitive.vertex_buffer.buffer],
                                    &[0],
                                );
                                device.handle.cmd_bind_index_buffer(
                                    command_buffer,
                                    mesh.primitive.index_buffer.buffer,
                                    0,
                                    vk::IndexType::UINT32,
                                );
                                device.handle.cmd_draw_indexed(
                                    command_buffer,
                                    mesh.primitive.indices.len() as u32,
                                    1,
                                    0,
                                    0,
                                    1,
                                );
                            }
                        }

                        device.handle.cmd_end_render_pass(command_buffer);
                    }

                    self.egui_integration
                        .context()
                        .set_visuals(egui::style::Visuals::dark());

                    let (_, shapes) = self.egui_integration.end_frame(&self.base.window);
                    let clipped_meshes = self.egui_integration.context().tessellate(shapes);
                    self.egui_integration.paint(
                        command_buffer,
                        present_index as usize,
                        clipped_meshes,
                    );
                },
            );

            self.base.submit_commands();
            self.base.present_frame(present_index);
        });
    }
}

fn main() {
    let mut app = Application::new();

    app.create_scene();
    app.run();
}
