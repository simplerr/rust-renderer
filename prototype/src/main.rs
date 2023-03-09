use ash::vk;
use glam::{Mat4, Vec3};

#[allow(dead_code)]
#[derive(Clone, Debug, Copy)]
struct ViewUniformData {
    view: glam::Mat4,
    projection: glam::Mat4,
    inverse_view: glam::Mat4,
    inverse_projection: glam::Mat4,
    eye_pos: glam::Vec3,
    samples_per_frame: u32,
    total_samples: u32,
    num_bounces: u32,
    viewport_width: u32,
    viewport_height: u32,
    sun_dir: glam::Vec3,
}

struct Application {
    base: utopian::VulkanBase,
    graph: utopian::Graph,
    view_data: ViewUniformData,
    camera_ubo: utopian::Buffer,
    camera: utopian::Camera,
    renderer: utopian::Renderer,
    raytracing: Option<utopian::Raytracing>,
    ui: prototype::ui::Ui,
    fps_timer: utopian::FpsTimer,
    raytracing_enabled: bool,
    profiling_enabled: bool,
    shader_watcher: utopian::DirectoryWatcher,
}

impl Application {
    fn new() -> Application {
        puffin::set_scopes_on(true);
        puffin::profile_function!();
        puffin::GlobalProfiler::lock().new_frame();

        let (width, height) = (2000, 1100);
        let base = utopian::VulkanBase::new(width, height);

        let renderer = utopian::Renderer::new(&base.device);

        let camera = utopian::Camera::new(
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, 0.5, 0.0),
            60.0,
            width as f32 / height as f32,
            0.01,
            20000.0,
            0.02,
        );

        // Move from here
        let view_data = ViewUniformData {
            view: camera.get_view(),
            projection: camera.get_projection(),
            inverse_view: camera.get_view().inverse(),
            inverse_projection: camera.get_projection().inverse(),
            eye_pos: camera.get_position(),
            samples_per_frame: 1,
            total_samples: 0,
            num_bounces: 5,
            viewport_width: width,
            viewport_height: height,
            sun_dir: Vec3::new(0.0, 0.9, 0.15).normalize(),
        };

        let slice = unsafe { std::slice::from_raw_parts(&view_data, 1) };

        // Move from here
        let camera_uniform_buffer = utopian::Buffer::new(
            &base.device,
            Some(slice),
            std::mem::size_of_val(&view_data) as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let ui = prototype::ui::Ui::new(
            width,
            height,
            &base.device,
            &base.swapchain_loader,
            base.swapchain,
            base.surface_format,
        );

        let shader_watcher = utopian::DirectoryWatcher::new("utopian/shaders/");

        let raytracing_supported = base.device.raytracing_supported;
        let raytracing = match raytracing_supported {
            true => Some(utopian::Raytracing::new(
                &base.device,
                base.surface_resolution,
                Some(renderer.bindless_descriptor_set_layout),
            )),
            false => None,
        };

        let mut graph = utopian::Graph::new(&base.device, &camera_uniform_buffer);

        utopian::renderers::build_render_graph(
            &mut graph,
            &base.device,
            &base,
            &renderer,
            &camera_uniform_buffer,
            view_data.sun_dir
        );

        Application {
            base,
            graph,
            view_data,
            camera_ubo: camera_uniform_buffer,
            camera,
            renderer,
            raytracing,
            ui,
            fps_timer: utopian::FpsTimer::new(),
            raytracing_enabled: raytracing_supported,
            profiling_enabled: false,
            shader_watcher,
        }
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

        prototype::scenes::create_scene(&mut self.renderer, &mut self.camera, &self.base.device);

        if let Some(raytracing) = &mut self.raytracing {
            raytracing.initialize(&self.base.device, &self.renderer.instances);
        }
    }

    fn update_ui(
        egui_context: &egui::Context,
        camera_pos: &mut Vec3,
        camera_dir: &mut Vec3,
        fps: u32,
        view_data: &mut ViewUniformData,
        selected_transform: &mut Mat4,
    ) {
        egui::Window::new("rust-renderer 0.0.1")
            .resizable(true)
            .vscroll(true)
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
                    ui.add(egui::widgets::Slider::new(
                        &mut view_data.samples_per_frame,
                        0..=10,
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("Num ray bounces:");
                    ui.add(egui::widgets::Slider::new(
                        &mut view_data.num_bounces,
                        0..=16,
                    ));
                });
                ui.label("Sun direction");
                ui.horizontal(|ui| {
                    ui.label("x:");
                    ui.add(egui::widgets::Slider::new(
                        &mut view_data.sun_dir.x,
                        0.0..=1.0,
                    ));
                    ui.label("y:");
                    ui.add(egui::widgets::Slider::new(
                        &mut view_data.sun_dir.y,
                        0.0..=1.0,
                    ));
                    ui.label("z:");
                    ui.add(egui::widgets::Slider::new(
                        &mut view_data.sun_dir.z,
                        0.0..=1.0,
                    ));
                });
            });

        egui::Area::new("Viewport")
            .fixed_pos((0.0, 0.0))
            .show(egui_context, |ui| {
                ui.with_layer_id(egui::LayerId::background(), |ui| {
                    let gizmo = egui_gizmo::Gizmo::new("Gizmo")
                        .view_matrix(view_data.view.to_cols_array_2d())
                        .projection_matrix(view_data.projection.to_cols_array_2d())
                        .model_matrix(selected_transform.to_cols_array_2d())
                        .mode(egui_gizmo::GizmoMode::Translate);

                    if let Some(response) = gizmo.interact(ui) {
                        *selected_transform = Mat4::from_cols_array_2d(&response.transform.into());
                    }
                });
            });
    }

    fn run(&mut self) {
        self.base.run(|input, events| unsafe {
            puffin::profile_scope!("main_run");

            let present_index = self.base.prepare_frame();

            self.ui.handle_events(events.clone(), self.base.window.id());
            self.ui.begin_frame();

            // Todo: refactor so not everything needs to be passed as argument
            let old_samples_per_frame = self.view_data.samples_per_frame;
            let old_num_bounces = self.view_data.num_bounces;
            let old_sun_dir = self.view_data.sun_dir;
            Application::update_ui(
                &self.ui.egui_integration.context(),
                &mut self.camera.get_position(),
                &mut self.camera.get_forward(),
                self.fps_timer.calculate(),
                &mut self.view_data,
                &mut self.renderer.instances[0].transform,
            );

            self.view_data.sun_dir = self.view_data.sun_dir.normalize();

            if self.view_data.samples_per_frame != old_samples_per_frame
                || self.view_data.num_bounces != old_num_bounces
                || self.view_data.sun_dir != old_sun_dir
            {
                self.view_data.total_samples = 0;
            }

            if input.key_pressed(winit::event::VirtualKeyCode::V) {
                self.raytracing_enabled =
                    !self.raytracing_enabled && self.base.device.raytracing_supported;
            }

            if self.shader_watcher.check_if_modification()
                || input.key_pressed(winit::event::VirtualKeyCode::R)
            {
                self.view_data.total_samples = 0;

                self.graph.recompile_shaders(
                    &self.base.device,
                    Some(self.renderer.bindless_descriptor_set_layout),
                );

                if let Some(raytracing) = &mut self.raytracing {
                    raytracing.recreate_pipeline(
                        &self.base.device,
                        Some(self.renderer.bindless_descriptor_set_layout),
                    );
                }
            }

            if input.key_pressed(winit::event::VirtualKeyCode::Q) {
                self.profiling_enabled = !self.profiling_enabled;
                puffin::set_scopes_on(self.profiling_enabled);
            }

            if self.camera.update(input) {
                self.view_data.total_samples = 0;
            }

            // Move from here
            self.view_data.view = self.camera.get_view();
            self.view_data.projection = self.camera.get_projection();
            self.view_data.inverse_view = self.camera.get_view().inverse();
            self.view_data.inverse_projection = self.camera.get_projection().inverse();
            self.view_data.eye_pos = self.camera.get_position();

            if self.raytracing_enabled {
                self.view_data.total_samples += self.view_data.samples_per_frame;
            }

            Application::record_commands(
                &self.base.device,
                self.base.draw_command_buffer,
                self.base.draw_commands_reuse_fence,
                |device, command_buffer| {
                    self.camera_ubo.update_memory(
                        &self.base.device,
                        std::slice::from_raw_parts(&self.view_data, 1),
                    );

                    if self.raytracing_enabled {
                        if let Some(raytracing) = &self.raytracing {
                            raytracing.record_commands(
                                device,
                                command_buffer,
                                self.renderer.bindless_descriptor_set,
                                self.graph.descriptor_set_camera.handle,
                                &self.base.present_images[present_index as usize],
                            );
                        }
                    } else {
                        // Remove passes from previous frame
                        self.graph.clear(&self.base.device);

                        // Build the render graph
                        utopian::renderers::build_render_graph(
                            &mut self.graph,
                            &self.base.device,
                            &self.base,
                            &self.renderer,
                            &self.camera_ubo,
                            self.view_data.sun_dir,
                        );

                        self.graph.prepare(device, &self.renderer);

                        self.graph.render(
                            device,
                            command_buffer,
                            &self.renderer,
                            &[self.base.present_images[present_index as usize].clone()],
                        );
                    }

                    if self.profiling_enabled {
                        puffin_egui::profiler_window(&self.ui.egui_integration.context());
                    }
                    puffin::GlobalProfiler::lock().new_frame();

                    // This also does the transition of the swapchain image to PRESENT_SRC_KHR
                    self.ui
                        .end_frame(command_buffer, present_index, &self.base.window);
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
