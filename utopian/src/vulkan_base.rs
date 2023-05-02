use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::Surface;
use ash::extensions::khr::Swapchain;
use ash::vk;

use std::borrow::Cow;
use std::cell::RefCell;
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::device::*;
use crate::image::*;
use crate::input::*;

// Simple offset_of macro akin to C++ offsetof
#[macro_export]
macro_rules! offset_of {
    ($base:path, $field:ident) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let b: $base = mem::zeroed();
            (&b.$field as *const _ as isize) - (&b as *const _ as isize)
        }
    }};
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number: i32 = callback_data.message_id_number as i32;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{:?}:\n{:?} [{} ({})] : {}\n",
        message_severity,
        message_type,
        message_id_name,
        &message_id_number.to_string(),
        message,
    );

    vk::FALSE
}

pub struct VulkanBase {
    pub window: winit::window::Window,
    event_loop: RefCell<winit::event_loop::EventLoop<()>>,
    pub instance: ash::Instance,
    pub device: Device,
    _command_pool: vk::CommandPool,
    pub draw_command_buffer: vk::CommandBuffer,
    pub present_images: Vec<Image>,
    pub depth_image: Image,
    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_loader: ash::extensions::khr::Swapchain,
    pub draw_commands_reuse_fence: vk::Fence,
    pub debug_callback: vk::DebugUtilsMessengerEXT,
}

impl VulkanBase {
    pub fn new(width: u32, height: u32) -> VulkanBase {
        let entry = ash::Entry::linked();

        let (window, event_loop) = VulkanBase::create_window(width, height);
        let instance = VulkanBase::create_instance(&entry, &window);
        let (debug_utils, debug_callback) = VulkanBase::create_debug_utils(&entry, &instance);

        let (surface, surface_loader) = VulkanBase::create_surface(&entry, &instance, &window);

        let device = Device::new(&instance, surface, &surface_loader, debug_utils);

        let (swapchain, swapchain_loader, surface_format, surface_resolution) =
            VulkanBase::create_swapchain(
                &instance,
                device.physical_device,
                &device.handle,
                surface,
                &surface_loader,
            );

        let (_command_pool, draw_command_buffer) =
            VulkanBase::create_command_buffer(&device.handle, device.queue_family_index);

        let (present_images, depth_image) = VulkanBase::setup_swapchain_images(
            &device,
            swapchain,
            &swapchain_loader,
            surface_format,
            surface_resolution,
        );

        let (present_complete_semaphore, rendering_complete_semaphore) =
            VulkanBase::create_semaphores(&device.handle);

        let fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        let draw_commands_reuse_fence = unsafe {
            device
                .handle
                .create_fence(&fence_create_info, None)
                .expect("Create fence failed.")
        };

        VulkanBase {
            window,
            event_loop: RefCell::new(event_loop),
            instance,
            device,
            _command_pool,
            draw_command_buffer,
            present_images,
            depth_image,
            present_complete_semaphore,
            rendering_complete_semaphore,
            surface_format,
            surface_resolution,
            swapchain,
            swapchain_loader,
            draw_commands_reuse_fence,
            debug_callback,
        }
    }

    fn create_window(
        width: u32,
        height: u32,
    ) -> (winit::window::Window, winit::event_loop::EventLoop<()>) {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("rust-renderer")
            .with_inner_size(winit::dpi::LogicalSize::new(
                f64::from(width),
                f64::from(height),
            ))
            .build(&event_loop)
            .expect("Failed to create window");

        (window, event_loop)
    }

    fn create_instance(entry: &ash::Entry, window: &winit::window::Window) -> ash::Instance {
        let app_name = CStr::from_bytes_with_nul(b"rust-renderer\0").unwrap();
        let layer_names = [CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap()];
        let layer_names_raw: Vec<*const c_char> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let surface_extensions = ash_window::enumerate_required_extensions(&window).unwrap();
        let mut extension_names_raw: Vec<*const i8> = Vec::from(surface_extensions);
        extension_names_raw.push(DebugUtils::name().as_ptr());

        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name)
            .application_version(0)
            .engine_name(app_name)
            .engine_version(0)
            .api_version(vk::make_api_version(0, 1, 3, 0));

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layer_names_raw)
            .enabled_extension_names(&extension_names_raw);

        unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("Failed to create Vulkan instance")
        }
    }

    fn create_debug_utils(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> (DebugUtils, vk::DebugUtilsMessengerEXT) {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                //vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_utils_loader = DebugUtils::new(entry, instance);

        let debug_utils_messenger = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap()
        };

        (debug_utils_loader, debug_utils_messenger)
    }

    fn create_surface(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &winit::window::Window,
    ) -> (vk::SurfaceKHR, Surface) {
        let surface = unsafe { ash_window::create_surface(entry, instance, window, None).unwrap() };
        let surface_loader = Surface::new(entry, instance);

        (surface, surface_loader)
    }

    fn create_swapchain(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: &ash::Device,
        surface: vk::SurfaceKHR,
        surface_loader: &Surface,
    ) -> (
        vk::SwapchainKHR,
        Swapchain,
        vk::SurfaceFormatKHR,
        vk::Extent2D,
    ) {
        unsafe {
            let surface_format = surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .expect("Error getting device surface formats")[0];

            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)
                .expect("Error getting device surface capabilities");

            let desired_image_count = surface_capabilities.min_image_count + 1;
            let surface_resolution = surface_capabilities.current_extent;
            let desired_transform = vk::SurfaceTransformFlagsKHR::IDENTITY;

            let present_modes = surface_loader
                .get_physical_device_surface_present_modes(physical_device, surface)
                .expect("Error getting present modes");
            let present_mode = present_modes
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .expect("Did not find expected present mode");

            let swapchain_loader = Swapchain::new(instance, device);

            let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
                .surface(surface)
                .min_image_count(desired_image_count)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(surface_resolution)
                .image_usage(
                    vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
                )
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(desired_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);

            let swapchain = swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap();

            (
                swapchain,
                swapchain_loader,
                surface_format,
                surface_resolution,
            )
        }
    }

    fn create_command_buffer(
        device: &ash::Device,
        queue_family_index: u32,
    ) -> (vk::CommandPool, vk::CommandBuffer) {
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_index);

        let pool = unsafe {
            device
                .create_command_pool(&pool_create_info, None)
                .expect("Failed to create command pool")
        };

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .expect("Failed to allocate command buffer")
        };

        (pool, command_buffers[0])
    }

    fn setup_swapchain_images(
        device: &Device,
        swapchain: vk::SwapchainKHR,
        swapchain_loader: &Swapchain,
        surface_format: vk::SurfaceFormatKHR,
        surface_resolution: vk::Extent2D,
    ) -> (Vec<Image>, Image) {
        unsafe {
            let present_images = swapchain_loader
                .get_swapchain_images(swapchain)
                .expect("Error getting swapchain images");

            let present_images: Vec<Image> = present_images
                .iter()
                .map(|&image| {
                    Image::new_from_handle(
                        device,
                        image,
                        ImageDesc::new_2d(
                            surface_resolution.width,
                            surface_resolution.height,
                            surface_format.format,
                        ),
                    )
                })
                .collect();

            let depth_image = Image::new_from_desc(
                device,
                ImageDesc::new_2d(
                    surface_resolution.width,
                    surface_resolution.height,
                    vk::Format::D32_SFLOAT,
                )
                .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                .aspect(vk::ImageAspectFlags::DEPTH),
            );

            device.execute_and_submit(|device, cb| {
                for present_image in &present_images {
                    crate::synch::image_pipeline_barrier(
                        device,
                        cb,
                        &present_image,
                        vk_sync::AccessType::Nothing,
                        vk_sync::AccessType::Present,
                    );
                }

                crate::synch::image_pipeline_barrier(
                    device,
                    cb,
                    &depth_image,
                    vk_sync::AccessType::Nothing,
                    vk_sync::AccessType::DepthStencilAttachmentWrite,
                );
            });

            (present_images, depth_image)
        }
    }

    fn create_semaphores(device: &ash::Device) -> (vk::Semaphore, vk::Semaphore) {
        unsafe {
            let semaphore_create_info = vk::SemaphoreCreateInfo::default();

            let present_complete_semaphore = device
                .create_semaphore(&semaphore_create_info, None)
                .expect("Error creating semaphore");
            let rendering_complete_semaphore = device
                .create_semaphore(&semaphore_create_info, None)
                .expect("Error creating semaphore");

            (present_complete_semaphore, rendering_complete_semaphore)
        }
    }

    // Replaced by dynamic rendering
    pub fn create_renderpass(base: &VulkanBase) -> vk::RenderPass {
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

    // Replaced by dynamic rendering
    pub fn create_framebuffers(
        base: &VulkanBase,
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

    pub fn prepare_frame(&self) -> u32 {
        unsafe {
            let (present_index, _) = self
                .swapchain_loader
                .acquire_next_image(
                    self.swapchain,
                    std::u64::MAX,
                    self.present_complete_semaphore,
                    vk::Fence::null(),
                )
                .expect("Error acquiring next swapchain image");

            present_index
        }
    }

    pub fn present_frame(&self, present_index: u32) {
        unsafe {
            let wait_semaphores = [self.rendering_complete_semaphore];
            let swapchains = [self.swapchain];
            let image_indices = [present_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&wait_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            self.swapchain_loader
                .queue_present(self.device.queue, &present_info)
                .unwrap();
        }
    }

    pub fn submit_commands(&self) {
        unsafe {
            let command_buffers = vec![self.draw_command_buffer];
            let wait_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let wait_semaphores = [self.present_complete_semaphore];
            let signal_semaphores = [self.rendering_complete_semaphore];

            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(&wait_semaphores)
                .signal_semaphores(&signal_semaphores)
                .wait_dst_stage_mask(&wait_mask)
                .command_buffers(&command_buffers);

            self.device
                .handle
                .queue_submit(
                    self.device.queue,
                    &[submit_info.build()],
                    self.draw_commands_reuse_fence,
                )
                .expect("Queue submit failed.");
        }
    }

    pub fn run<F: FnMut(&Input, &Vec<WindowEvent<'static>>)>(&self, mut user_function: F) {
        let mut events = Vec::new();
        let mut input = Input::default();

        let mut running = true;
        while running {
            self.event_loop
                .borrow_mut()
                .run_return(|event, _, control_flow| {
                    *control_flow = ControlFlow::Poll;

                    match event {
                        Event::WindowEvent { event, window_id }
                            if window_id == self.window.id() =>
                        {
                            match event {
                                WindowEvent::CloseRequested => {
                                    *control_flow = ControlFlow::Exit;
                                    running = false;
                                }
                                _ => events.extend(event.to_static()),
                            }
                        }
                        Event::MainEventsCleared => {
                            *control_flow = ControlFlow::Exit;
                        }
                        _ => (),
                    }
                });

            input.update(&events);

            user_function(&input, &events);

            events.clear();
        }
    }
}
