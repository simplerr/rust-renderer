use ash::extensions::khr;
use ash::extensions::khr::Surface;
use ash::extensions::khr::Swapchain;
use ash::vk;

pub struct Device {
    pub handle: ash::Device,
    pub physical_device: vk::PhysicalDevice,
    pub queue: vk::Queue,
    pub cmd_pool: vk::CommandPool,
    pub setup_cmd_buf: vk::CommandBuffer,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub queue_family_index: u32,
    pub rt_pipeline_properties: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
    pub acceleration_structure_ext: khr::AccelerationStructure,
    pub raytracing_pipeline_ext: khr::RayTracingPipeline,
}

impl Device {
    pub fn new(
        instance: &ash::Instance,
        surface: vk::SurfaceKHR,
        surface_loader: &Surface,
    ) -> Device {
        unsafe {
            let physical_devices = instance
                .enumerate_physical_devices()
                .expect("Error enumerating physical devices");

            // Note: assume single physical device
            let physical_device = physical_devices[0];

            let queue_family_properties =
                instance.get_physical_device_queue_family_properties(physical_device);
            let queue_family_index = queue_family_properties
                .iter()
                .position(|info| info.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                .expect("Did not find any matching graphics queue");
            let queue_family_index = queue_family_index as u32;
            surface_loader
                .get_physical_device_surface_support(physical_device, queue_family_index, surface)
                .expect("Presentation of the queue family not supported by the surface");

            let mut device_extension_names_raw = vec![
                Swapchain::name().as_ptr(),
                vk::ExtDescriptorIndexingFn::name().as_ptr(),
            ];

            device_extension_names_raw.extend([
                vk::KhrAccelerationStructureFn::name().as_ptr(),
                vk::KhrBufferDeviceAddressFn::name().as_ptr(),
                vk::KhrRayTracingPipelineFn::name().as_ptr(),
                vk::KhrDeferredHostOperationsFn::name().as_ptr(),
                vk::KhrSpirv14Fn::name().as_ptr(),
                vk::KhrShaderFloatControlsFn::name().as_ptr(),
                vk::ExtScalarBlockLayoutFn::name().as_ptr(),
            ]);

            let mut descriptor_indexing_features =
                vk::PhysicalDeviceDescriptorIndexingFeaturesEXT::default();
            let mut ray_tracing_pipeline_features =
                vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default();
            let mut acceleration_structure_features =
                vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default();
            let mut buffer_device_address_features =
                vk::PhysicalDeviceBufferDeviceAddressFeaturesKHR::default();
            let mut scalar_block_layout_features =
                vk::PhysicalDeviceScalarBlockLayoutFeatures::default();

            let mut features2 = vk::PhysicalDeviceFeatures2::builder()
                .push_next(&mut descriptor_indexing_features)
                .push_next(&mut ray_tracing_pipeline_features)
                .push_next(&mut acceleration_structure_features)
                .push_next(&mut buffer_device_address_features)
                .push_next(&mut scalar_block_layout_features)
                .build();

            instance.get_physical_device_features2(physical_device, &mut features2);

            let queue_priorities = [1.0];
            let queue_info = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .queue_priorities(&queue_priorities);

            let device_create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(std::slice::from_ref(&queue_info))
                .enabled_extension_names(&device_extension_names_raw)
                .push_next(&mut features2);

            let device: ash::Device = instance
                .create_device(physical_device, &device_create_info, None)
                .expect("Failed to create logical Vulkan device");

            let present_queue = device.get_device_queue(queue_family_index, 0);

            let device_memory_properties =
                instance.get_physical_device_memory_properties(physical_device);

            let (cmd_pool, setup_cmd_buf) =
                Device::create_setup_command_buffer(&device, queue_family_index);

            let (rt_pipeline_properties, as_features) =
                Device::retrieve_rt_properties(instance, physical_device);

            let acceleration_structure_ext = khr::AccelerationStructure::new(instance, &device);
            let raytracing_pipeline_ext = khr::RayTracingPipeline::new(instance, &device);

            println!("{:#?}", rt_pipeline_properties);
            println!("{:#?}", as_features);

            Device {
                handle: device,
                physical_device,
                queue: present_queue,
                queue_family_index,
                device_memory_properties,
                cmd_pool,
                setup_cmd_buf,
                rt_pipeline_properties,
                acceleration_structure_ext,
                raytracing_pipeline_ext,
            }
        }
    }

    fn retrieve_rt_properties(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> (
        vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
        vk::PhysicalDeviceAccelerationStructureFeaturesKHR,
    ) {
        unsafe {
            let mut rt_pipeline_properties =
                vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
            let mut properties2 = vk::PhysicalDeviceProperties2::builder()
                .push_next(&mut rt_pipeline_properties)
                .build();
            instance.get_physical_device_properties2(physical_device, &mut properties2);

            let mut acceleration_structure_features =
                vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default();
            let mut features2 = vk::PhysicalDeviceFeatures2::builder()
                .push_next(&mut acceleration_structure_features)
                .build();
            instance.get_physical_device_features2(physical_device, &mut features2);

            (rt_pipeline_properties, acceleration_structure_features)
        }
    }

    fn create_setup_command_buffer(
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

    pub fn execute_and_submit<F: FnOnce(&Device, vk::CommandBuffer)>(&self, recording_function: F) {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            self.handle
                .begin_command_buffer(self.setup_cmd_buf, &command_buffer_begin_info)
                .expect("Begin command buffer failed.")
        };

        recording_function(self, self.setup_cmd_buf);

        unsafe {
            self.handle
                .end_command_buffer(self.setup_cmd_buf)
                .expect("End commandbuffer failed.")
        };

        let submit_info =
            vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&self.setup_cmd_buf));

        unsafe {
            self.handle
                .queue_submit(self.queue, &[submit_info.build()], vk::Fence::null())
                .expect("Queue submit failed");

            self.handle
                .device_wait_idle()
                .expect("Device wait idle failed");
        }
    }

    pub fn find_memory_type_index(
        &self,
        memory_req: &vk::MemoryRequirements,
        flags: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        self.device_memory_properties.memory_types
            [..self.device_memory_properties.memory_type_count as _]
            .iter()
            .enumerate()
            .find(|(index, memory_type)| {
                (1 << index) & memory_req.memory_type_bits != 0
                    && memory_type.property_flags & flags == flags
            })
            .map(|(index, _memory_type)| index as _)
    }
}
