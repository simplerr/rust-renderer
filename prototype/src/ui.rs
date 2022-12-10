
use std::sync::Arc;
use std::sync::Mutex;
use ash::vk;

pub struct Ui {
    pub egui_integration:
        egui_winit_ash_integration::Integration<Arc<Mutex<gpu_allocator::vulkan::Allocator>>>,
}

impl Ui {
    pub fn new(
        width: u32,
        height: u32,
        device: &utopian::Device,
        swapchain_loader: &ash::extensions::khr::Swapchain,
        swapchain: vk::SwapchainKHR,
        surface_format: vk::SurfaceFormatKHR,
    ) -> Self {
        let egui_integration = egui_winit_ash_integration::Integration::new(
            width,
            height,
            1.0,
            egui::FontDefinitions::default(),
            egui::Style::default(),
            device.handle.clone(),
            device.gpu_allocator.clone(),
            swapchain_loader.clone(),
            swapchain,
            surface_format,
        );

        Ui {
            egui_integration
        }
    }

    pub fn handle_events(&mut self, events: Vec<winit::event::WindowEvent>, window_id: winit::window::WindowId) {
        // Update egui input
        // Note: this is not very pretty since we are recreating a Event from
        // an WindowEvent manually. Don't know enough rust to have the `events`
        // input of the correct type.
        for event in events {
            self.egui_integration
                .handle_event::<winit::event::Event<winit::event::WindowEvent>>(
                    &winit::event::Event::WindowEvent {
                        window_id,
                        event: event,
                    },
                );
        }
    }

    pub fn begin_frame(&mut self) {
        self.egui_integration.begin_frame();
    }

    pub fn end_frame(&mut self, command_buffer: vk::CommandBuffer, present_index: u32, window: &winit::window::Window) {
        self.egui_integration
            .context()
            .set_visuals(egui::style::Visuals::dark());

        let (_, shapes) = self.egui_integration.end_frame(window);
        let clipped_meshes = self.egui_integration.context().tessellate(shapes);
        self.egui_integration.paint(
            // This also does the transition of the swapchain image to PRESENT_SRC_KHR
            command_buffer,
            present_index as usize,
            clipped_meshes,
        );
    }
}