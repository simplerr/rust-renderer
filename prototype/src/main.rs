use ash::vk;
use glam::{Mat4, Vec3};
use prototype::ui::U32Checkbox;

struct Application {
    base: utopian::VulkanBase,
    graph: utopian::Graph,
    view_data: utopian::ViewUniformData,
    camera_ubo: utopian::Buffer,
    camera: utopian::Camera,
    renderer: utopian::Renderer,
    ui: prototype::ui::Ui,
    fps_timer: utopian::FpsTimer,
    raytracing_enabled: bool,
    shader_watcher: utopian::DirectoryWatcher,
}

impl Application {
    fn new() -> Application {
        puffin::set_scopes_on(true);
        puffin::profile_function!();
        puffin::GlobalProfiler::lock().new_frame();

        let (width, height) = (2000, 1100);
        let base = utopian::VulkanBase::new(width, height);

        let renderer = utopian::Renderer::new(&base.device, width, height);

        let camera = utopian::Camera::new(
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, 0.5, 0.0),
            60.0,
            width as f32 / height as f32,
            0.01,
            1000.0,
            0.20,
        );

        // Move from here
        let view_data = utopian::ViewUniformData {
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
            time: 0.0,
            sun_dir: Vec3::new(0.0, 0.9, 0.15).normalize(),
            shadows_enabled: 1,
            ssao_enabled: 1,
            fxaa_enabled: 1,
            cubemap_enabled: 1,
            ibl_enabled: 1,
            marching_cubes_enabled: 0,
            rebuild_tlas: 1,
        };

        // Move from here
        let camera_uniform_buffer = utopian::Buffer::new(
            &base.device,
            Some(std::slice::from_ref(&view_data)),
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

        let graph = utopian::Graph::new(&base.device, &camera_uniform_buffer);

        Application {
            base,
            graph,
            view_data,
            camera_ubo: camera_uniform_buffer,
            camera,
            renderer,
            ui,
            fps_timer: utopian::FpsTimer::new(),
            raytracing_enabled: raytracing_supported,
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
            {
                puffin::profile_scope!("wait_for_fences");
                device
                    .handle
                    .wait_for_fences(&[wait_fence], true, std::u64::MAX)
                    .expect("Wait for fence failed.");

                device
                    .handle
                    .reset_fences(&[wait_fence])
                    .expect("Reset fences failed.");
            }

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

        if let Some(raytracing) = &mut self.renderer.raytracing {
            raytracing.initialize(&self.base.device, &self.renderer.instances);
        }
    }

    fn update_ui(
        egui_context: &egui::Context,
        camera_pos: &mut Vec3,
        camera_dir: &mut Vec3,
        fps: u32,
        view_data: &mut utopian::ViewUniformData,
        selected_transform: &mut Mat4,
        need_environment_map_update: &mut bool,
    ) {
        egui::Window::new("rust-renderer 0.0.1")
            .resizable(true)
            .vscroll(true)
            .show(egui_context, |ui| {
                ui.label(format!("FPS: {} ({} ms)", fps, 1000.0 / fps as f32));
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
                    egui::Grid::new("sun_dir").show(ui, |ui| {
                        ui.label("x:");
                        ui.add(egui::widgets::Slider::new(
                            &mut view_data.sun_dir.x,
                            -1.0..=1.0,
                        ));
                        ui.end_row();
                        ui.label("y:");
                        ui.add(egui::widgets::Slider::new(
                            &mut view_data.sun_dir.y,
                            -1.0..=1.0,
                        ));
                        ui.end_row();
                        ui.label("z:");
                        ui.add(egui::widgets::Slider::new(
                            &mut view_data.sun_dir.z,
                            -1.0..=1.0,
                        ));
                    });
                });
                ui.horizontal(|ui| {
                    // Bloated code due to needing u32 since view data is a uniform buffer
                    egui::Grid::new("settings_grid").show(ui, |ui| {
                        ui.add(U32Checkbox::new(&mut view_data.shadows_enabled, "Shadows:"));
                        ui.add(U32Checkbox::new(&mut view_data.ssao_enabled, "SSAO:"));
                        ui.add(U32Checkbox::new(&mut view_data.fxaa_enabled, "FXAA:"));
                        ui.add(U32Checkbox::new(&mut view_data.cubemap_enabled, "Cubemap:"));
                        ui.add(U32Checkbox::new(&mut view_data.ibl_enabled, "IBL:"));
                        ui.add(U32Checkbox::new(
                            &mut view_data.rebuild_tlas,
                            "Rebuild TLAS:",
                        ));
                        ui.add(U32Checkbox::new(
                            &mut view_data.marching_cubes_enabled,
                            "Marching cubes:",
                        ));

                        if ui.button("Generate environment map").clicked() {
                            *need_environment_map_update = true;
                        }
                    });
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
                        view_data.total_samples = 0; // Reset raytracing when moving objects
                    }
                });
            });
    }

    fn run(&mut self) {
        self.base.run(|input, events| {
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
                &mut self.renderer.instances[1].transform,
                &mut self.renderer.need_environment_map_update,
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
                self.view_data.total_samples = 0;
            }

            if let Some(path) = self.shader_watcher.check_if_modification() {
                //     || input.key_pressed(winit::event::VirtualKeyCode::R)
                self.view_data.total_samples = 0;

                if let Some(ext) = path.extension() {
                    let mut recompile_rt_shaders = false;
                    if ext == "glsl" {
                        self.graph.recompile_all_shaders(
                            &self.base.device,
                            Some(self.renderer.bindless_descriptor_set_layout),
                        );
                        recompile_rt_shaders = true;
                    } else {
                        self.graph.recompile_shader(
                            &self.base.device,
                            Some(self.renderer.bindless_descriptor_set_layout),
                            path.clone(),
                        );
                    }

                    if ["rchit", "rgen", "rmiss"].contains(&ext.to_str().unwrap())
                        || recompile_rt_shaders
                    {
                        if let Some(raytracing) = &mut self.renderer.raytracing {
                            raytracing.recreate_pipeline(
                                &self.base.device,
                                Some(self.renderer.bindless_descriptor_set_layout),
                            );
                        }
                    }
                }
            }

            if input.key_pressed(winit::event::VirtualKeyCode::Q) {
                self.graph.profiling_enabled = !self.graph.profiling_enabled;
                puffin::set_scopes_on(self.graph.profiling_enabled);
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
            self.view_data.time = self.fps_timer.elapsed_seconds_from_start();

            if self.raytracing_enabled {
                self.view_data.total_samples += self.view_data.samples_per_frame;
            }

            Application::record_commands(
                &self.base.device,
                self.base.draw_command_buffer,
                self.base.draw_commands_reuse_fence,
                |device, command_buffer| {
                    self.camera_ubo
                        .update_memory(&self.base.device, std::slice::from_ref(&self.view_data));

                    let gpu_frame_start_ns = if self.graph.profiling_enabled {
                        gpu_profiler::profiler().begin_frame();
                        puffin::now_ns()
                    } else {
                        0
                    };

                    // Remove passes from previous frame
                    self.graph.clear(&self.base.device);

                    if self.raytracing_enabled {
                        utopian::renderers::build_path_tracing_render_graph(
                            &mut self.graph,
                            &self.base.device,
                            &self.base,
                        );
                    } else {
                        utopian::renderers::build_render_graph(
                            &mut self.graph,
                            &self.base.device,
                            &self.base,
                            &self.renderer,
                            &self.view_data,
                            &self.camera,
                        );

                        // Todo: should be possible to trigger this when needed
                        self.renderer.need_environment_map_update = false;
                    }

                    self.graph.prepare(device, &self.renderer);

                    self.graph.render(
                        device,
                        command_buffer,
                        &mut self.renderer,
                        &[self.base.present_images[present_index as usize].clone()],
                        self.view_data.rebuild_tlas == 1,
                    );

                    if self.graph.profiling_enabled {
                        gpu_profiler::profiler().end_frame();
                        if let Some(report) = gpu_profiler::profiler().last_report() {
                            report.send_to_puffin(gpu_frame_start_ns);
                        };
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
