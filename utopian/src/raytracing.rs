use ash::vk;
use glam::{Mat3, Vec3, Mat4};
use std::ffi::CStr;
use std::io::Cursor;
use std::mem;

use crate::buffer::*;
use crate::descriptor_set::*;
use crate::device::*;
use crate::image::*;
use crate::primitive::*;
use crate::renderer::*;
use crate::shader::*;

pub struct Raytracing {
    top_level_acceleration: vk::AccelerationStructureKHR,
    bottom_level_accelerations: Vec<vk::AccelerationStructureKHR>,
    storage_image: Image,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    raygen_sbt_buffer: Buffer,
    miss_sbt_buffer: Buffer,
    hit_sbt_buffer: Buffer,
    descriptor_set: DescriptorSet,
}

impl Raytracing {
    pub fn new(device: &Device, camera_uniform_buffer: &Buffer) -> Self {
        let storage_image = Raytracing::create_storage_image(device, 2000, 1100);

        let (pipeline, reflection, pipeline_layout, descriptor_set_layouts) =
            Raytracing::create_pipeline(
                device,
                "utopian/shaders/raytracing_basic/basic.rgen",
                "utopian/shaders/raytracing_basic/basic.rmiss",
                "utopian/shaders/raytracing_basic/basic.rchit",
            );

        let (raygen_sbt_buffer, miss_sbt_buffer, hit_sbt_buffer) =
            Raytracing::create_shader_binding_table(device, pipeline);

        let binding = reflection.get_binding("topLevelAS");

        let descriptor_set = DescriptorSet::new(
            device,
            descriptor_set_layouts[binding.set as usize],
            reflection.get_set_mappings(binding.set),
        );

        descriptor_set.write_uniform_buffer(&device, "camera".to_string(), &camera_uniform_buffer);
        descriptor_set.write_storage_image(&device, "image".to_string(), &storage_image);

        Raytracing {
            bottom_level_accelerations: vec![],
            top_level_acceleration: vk::AccelerationStructureKHR::null(),
            storage_image,
            pipeline,
            pipeline_layout,
            raygen_sbt_buffer,
            miss_sbt_buffer,
            hit_sbt_buffer,
            descriptor_set,
        }
    }

    pub fn initialize(&mut self, device: &Device, instances: &Vec<ModelInstance>) {
        for instance in instances {
            for mesh in &instance.model.meshes {
                self.bottom_level_accelerations.push(
                    Raytracing::create_bottom_level_acceleration_structure(device, &mesh.primitive),
                );
            }
        }

        let tlas = Raytracing::create_top_level_acceleration_structure(
            device,
            &self.bottom_level_accelerations,
            &instances,
        );

        self.descriptor_set
            .write_acceleration_structure(&device, "topLevelAS".to_string(), tlas);

        self.top_level_acceleration = tlas;
    }

    pub fn create_bottom_level_acceleration_structure(
        device: &Device,
        primitive: &Primitive,
    ) -> vk::AccelerationStructureKHR {
        let vertex_buffer_device_address = vk::DeviceOrHostAddressConstKHR {
            device_address: primitive.vertex_buffer.get_device_address(device),
        };
        let index_buffer_device_address = vk::DeviceOrHostAddressConstKHR {
            device_address: primitive.index_buffer.get_device_address(device),
        };

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .flags(vk::GeometryFlagsKHR::OPAQUE)
            .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                triangles: vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
                    .vertex_format(vk::Format::R32G32B32_SFLOAT)
                    .vertex_data(vertex_buffer_device_address)
                    .vertex_stride(mem::size_of::<Vertex>() as _)
                    .max_vertex(primitive.vertices.len() as _)
                    .index_type(vk::IndexType::UINT32)
                    .index_data(index_buffer_device_address)
                    .build(),
            })
            .build();

        // Get size info
        let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(ash::vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .geometries(std::slice::from_ref(&geometry))
            .build();

        let num_triangles = (primitive.indices.len() / 3) as u32;

        let build_sizes = unsafe {
            device
                .acceleration_structure_ext
                .get_acceleration_structure_build_sizes(
                    vk::AccelerationStructureBuildTypeKHR::DEVICE,
                    &build_geometry_info,
                    &[num_triangles],
                )
        };

        // Todo: this should be created using device local memory
        let blas_buffer = Buffer::new(
            device,
            &[0],
            build_sizes.acceleration_structure_size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR,
        );

        let create_info = vk::AccelerationStructureCreateInfoKHR::builder()
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .buffer(blas_buffer.buffer)
            .size(build_sizes.acceleration_structure_size)
            .build();

        let acceleration_structure = unsafe {
            device
                .acceleration_structure_ext
                .create_acceleration_structure(&create_info, None)
                .expect("Creation of acceleration structure failed")
        };

        let scratch_buffer = Buffer::new(
            device,
            &[0],
            build_sizes.build_scratch_size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::STORAGE_BUFFER,
        );

        let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(ash::vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .geometries(std::slice::from_ref(&geometry))
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .dst_acceleration_structure(acceleration_structure)
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: scratch_buffer.get_device_address(device),
            })
            .build();

        let build_range_info = vec![ash::vk::AccelerationStructureBuildRangeInfoKHR::builder()
            .primitive_count(num_triangles)
            .build()];

        unsafe {
            device.execute_and_submit(|device, cb| {
                device
                    .acceleration_structure_ext
                    .cmd_build_acceleration_structures(
                        cb,
                        std::slice::from_ref(&build_geometry_info),
                        std::slice::from_ref(&build_range_info.as_slice()),
                    );
            });
        }

        acceleration_structure
    }

    pub fn create_top_level_acceleration_structure(
        device: &Device,
        blas: &Vec<vk::AccelerationStructureKHR>,
        instances: &Vec<ModelInstance>,
    ) -> vk::AccelerationStructureKHR {
        let mut acceleration_instances: Vec<vk::AccelerationStructureInstanceKHR> = vec![];
        let mut blas_idx = 0;
        for instance in instances {
            for (i, _mesh) in instance.model.meshes.iter().enumerate() {
                let world_matrix = instance.transform * instance.model.transforms[i];
                let (scale, rotation, translation) = world_matrix.to_scale_rotation_translation();
                let rotation_matrix = Mat3::from_quat(rotation);

                let transform = vk::TransformMatrixKHR {
                    matrix: [
                        rotation_matrix.x_axis.x * scale.x,
                        rotation_matrix.y_axis.x * scale.y,
                        rotation_matrix.z_axis.x * scale.z,
                        translation.x,
                        rotation_matrix.x_axis.y * scale.x,
                        rotation_matrix.y_axis.y * scale.y,
                        rotation_matrix.z_axis.y * scale.z,
                        translation.y,
                        rotation_matrix.x_axis.z * scale.x,
                        rotation_matrix.y_axis.z * scale.y,
                        rotation_matrix.z_axis.z * scale.z,
                        translation.z,
                    ],
                };

                let blas_device_address = unsafe {
                    device
                        .acceleration_structure_ext
                        .get_acceleration_structure_device_address(
                            &vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                                .acceleration_structure(blas[blas_idx])
                                .build(),
                        )
                };

                acceleration_instances.push(vk::AccelerationStructureInstanceKHR {
                    transform,
                    acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                        device_handle: blas_device_address,
                    },
                    instance_custom_index_and_mask: vk::Packed24_8::new(0, 0xff),
                    instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                        0,
                        vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as u8,
                    ),
                });

                blas_idx += 1;
            }
        }

        let instances_buffer = Buffer::new(
            device,
            acceleration_instances.as_slice(),
            std::mem::size_of_val(&*acceleration_instances) as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
        );

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .flags(vk::GeometryFlagsKHR::OPAQUE)
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: vk::AccelerationStructureGeometryInstancesDataKHR::builder()
                    .array_of_pointers(false)
                    .data(vk::DeviceOrHostAddressConstKHR {
                        device_address: instances_buffer.get_device_address(device),
                    })
                    .build(),
            })
            .build();

        // Get size info
        let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(ash::vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .geometries(std::slice::from_ref(&geometry))
            .build();

        let num_instances = acceleration_instances.len() as u32;

        let build_sizes = unsafe {
            device
                .acceleration_structure_ext
                .get_acceleration_structure_build_sizes(
                    vk::AccelerationStructureBuildTypeKHR::DEVICE,
                    &build_geometry_info,
                    &[num_instances],
                )
        };

        // Todo: this should be created using device local memory
        let tlas_buffer = Buffer::new(
            device,
            &[0],
            build_sizes.acceleration_structure_size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR,
        );

        let create_info = vk::AccelerationStructureCreateInfoKHR::builder()
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .buffer(tlas_buffer.buffer)
            .size(build_sizes.acceleration_structure_size)
            .build();

        let acceleration_structure = unsafe {
            device
                .acceleration_structure_ext
                .create_acceleration_structure(&create_info, None)
                .expect("Creation of acceleration structure failed")
        };

        let scratch_buffer = Buffer::new(
            device,
            &[0],
            build_sizes.build_scratch_size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::STORAGE_BUFFER,
        );

        let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(ash::vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .geometries(std::slice::from_ref(&geometry))
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .dst_acceleration_structure(acceleration_structure)
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: scratch_buffer.get_device_address(device),
            })
            .build();

        let build_range_info = vec![ash::vk::AccelerationStructureBuildRangeInfoKHR::builder()
            .primitive_count(num_instances)
            .build()];

        unsafe {
            device.execute_and_submit(|device, cb| {
                device
                    .acceleration_structure_ext
                    .cmd_build_acceleration_structures(
                        cb,
                        std::slice::from_ref(&build_geometry_info),
                        std::slice::from_ref(&build_range_info.as_slice()),
                    );
            });
        }

        acceleration_structure

        // Todo: cleanup scratch buffer and instances buffer
    }

    pub fn create_storage_image(device: &Device, width: u32, height: u32) -> Image {
        let storage_image = Image::new(
            device,
            width,
            height,
            vk::Format::B8G8R8A8_UNORM, // Matches the swapchain image format
            vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::STORAGE,
            vk::ImageAspectFlags::COLOR,
        );

        device.execute_and_submit(|device, cb| {
            storage_image.transition_layout(device, cb, vk::ImageLayout::GENERAL);
        });

        storage_image
    }

    pub fn create_pipeline(
        device: &Device,
        raygen_shader_path: &str,
        miss_shader_path: &str,
        closest_hit_shader_path: &str,
    ) -> (
        vk::Pipeline,
        Reflection,
        vk::PipelineLayout,
        Vec<vk::DescriptorSetLayout>,
    ) {
        let raygen_spv_file = compile_glsl_shader(raygen_shader_path);
        let miss_spv_file = compile_glsl_shader(miss_shader_path);
        let closest_hit_spv_file = compile_glsl_shader(closest_hit_shader_path);

        let raygen_spv_file = raygen_spv_file.as_binary_u8();
        let miss_spv_file = miss_spv_file.as_binary_u8();
        let closest_hit_spv_file = closest_hit_spv_file.as_binary_u8();

        let reflection = Reflection::new(&[raygen_spv_file, miss_spv_file, closest_hit_spv_file]);
        let (pipeline_layout, descriptor_set_layouts, _) =
            create_layouts_from_reflection(&device.handle, &reflection, None);

        let raygen_spv_file = Cursor::new(raygen_spv_file);
        let miss_spv_file = Cursor::new(miss_spv_file);
        let closest_hit_spv_file = Cursor::new(closest_hit_spv_file);

        let raygen_shader_module =
            crate::shader::create_shader_module(raygen_spv_file, &device.handle);
        let miss_shader_module = crate::shader::create_shader_module(miss_spv_file, &device.handle);
        let closest_hit_shader_module =
            crate::shader::create_shader_module(closest_hit_spv_file, &device.handle);

        let shader_entry_name = CStr::from_bytes_with_nul(b"main\0").unwrap();
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo {
                module: raygen_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::RAYGEN_KHR,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: miss_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::MISS_KHR,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: closest_hit_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::CLOSEST_HIT_KHR,
                ..Default::default()
            },
        ];

        let shader_group_create_infos = [
            ash::vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(ash::vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(0) // Todo: not hardcode like this
                .closest_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .any_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .intersection_shader(ash::vk::SHADER_UNUSED_KHR)
                .build(),
            ash::vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(ash::vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(1) // Todo: not hardcode like this
                .closest_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .any_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .intersection_shader(ash::vk::SHADER_UNUSED_KHR)
                .build(),
            ash::vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(ash::vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
                .general_shader(ash::vk::SHADER_UNUSED_KHR) // Todo: not hardcode like this
                .closest_hit_shader(2)
                .any_hit_shader(ash::vk::SHADER_UNUSED_KHR)
                .intersection_shader(ash::vk::SHADER_UNUSED_KHR)
                .build(),
        ];

        let pipeline_create_info = vk::RayTracingPipelineCreateInfoKHR::builder()
            .max_pipeline_ray_recursion_depth(1)
            .layout(pipeline_layout)
            .stages(&shader_stage_create_infos)
            .groups(&shader_group_create_infos)
            .build();

        let pipeline = unsafe {
            device
                .raytracing_pipeline_ext
                .create_ray_tracing_pipelines(
                    vk::DeferredOperationKHR::null(),
                    vk::PipelineCache::null(),
                    &[pipeline_create_info],
                    None,
                )
                .expect("Failed to create raytracing pipeline")[0]
        };

        (
            pipeline,
            reflection,
            pipeline_layout,
            descriptor_set_layouts,
        )
    }

    pub fn create_shader_binding_table(
        device: &Device,
        pipeline: vk::Pipeline,
    ) -> (Buffer, Buffer, Buffer) {
        let handle_size = device.rt_pipeline_properties.shader_group_handle_size as usize;
        let group_count = 3; // alignment? note that the size corresponds to shader_group_create_infos
        let sbt_size = group_count * handle_size;

        let shader_handle_storage = unsafe {
            device
                .raytracing_pipeline_ext
                .get_ray_tracing_shader_group_handles(
                    pipeline,
                    0,
                    group_count as u32,
                    sbt_size as usize,
                )
                .expect("Failed to get raytracing shader group handles")
        };

        let raygen_sbt_buffer = Buffer::new(
            device,
            &shader_handle_storage[0..handle_size],
            handle_size as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR,
        );

        let miss_sbt_buffer = Buffer::new(
            device,
            &shader_handle_storage[handle_size..handle_size * 2],
            handle_size as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR,
        );

        let hit_sbt_buffer = Buffer::new(
            device,
            &shader_handle_storage[handle_size * 2..handle_size * 3],
            handle_size as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR,
        );

        (raygen_sbt_buffer, miss_sbt_buffer, hit_sbt_buffer)
    }

    pub fn record_commands(&self, device: &Device, cb: vk::CommandBuffer, present_image: &Image) {
        let raygen_shader_binding_table = vk::StridedDeviceAddressRegionKHR {
            device_address: self.raygen_sbt_buffer.get_device_address(device),
            stride: device.rt_pipeline_properties.shader_group_handle_size as u64,
            size: device.rt_pipeline_properties.shader_group_handle_size as u64,
        };

        let miss_shader_binding_table = vk::StridedDeviceAddressRegionKHR {
            device_address: self.miss_sbt_buffer.get_device_address(device),
            stride: device.rt_pipeline_properties.shader_group_handle_size as u64,
            size: device.rt_pipeline_properties.shader_group_handle_size as u64,
        };

        let hit_shader_binding_table = vk::StridedDeviceAddressRegionKHR {
            device_address: self.hit_sbt_buffer.get_device_address(device),
            stride: device.rt_pipeline_properties.shader_group_handle_size as u64,
            size: device.rt_pipeline_properties.shader_group_handle_size as u64,
        };

        let callable_shader_binding_table = vk::StridedDeviceAddressRegionKHR {
            device_address: Default::default(),
            stride: 0,
            size: 0,
        };

        unsafe {
            device.handle.cmd_bind_pipeline(
                cb,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.pipeline,
            );
            device.handle.cmd_bind_descriptor_sets(
                cb,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.pipeline_layout,
                0,
                &[self.descriptor_set.handle],
                &[],
            );

            device.raytracing_pipeline_ext.cmd_trace_rays(
                cb,
                &raygen_shader_binding_table,
                &miss_shader_binding_table,
                &hit_shader_binding_table,
                &callable_shader_binding_table,
                2000,
                1100,
                1,
            );

            present_image.transition_layout(device, cb, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
            self.storage_image
                .transition_layout(device, cb, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);

            self.storage_image.copy(device, cb, &present_image);

            present_image.transition_layout(device, cb, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
            self.storage_image
                .transition_layout(device, cb, vk::ImageLayout::GENERAL);
        }
    }
}
