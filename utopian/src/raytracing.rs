use ash::vk;
use glam::Mat3;
use std::mem;

use crate::buffer::*;
use crate::descriptor_set::DescriptorIdentifier;
use crate::descriptor_set::*;
use crate::device::*;
use crate::image::*;
use crate::primitive::*;
use crate::renderer::*;

// The buffer sizes exactly fits the instance transform for all objects
// added to the scene when `Raytracing::initialize` is called.
// Todo: support adding/removing objects to the scene
pub struct Tlas {
    pub handle: vk::AccelerationStructureKHR,
    instances_buffer: Buffer,
    scratch_buffer: Buffer,
    // `Buffer::update_memory` creates a temporary staging buffer which is expensive
    // so we have a persistent staging buffer here for now
    staging_instances_buffer: Buffer,
}

pub struct Raytracing {
    pub top_level_acceleration: Option<Tlas>,
    bottom_level_accelerations: Vec<vk::AccelerationStructureKHR>,
    output_image: Image,
    _accumulation_image: Image,
    pipeline: crate::Pipeline,
    descriptor_set: DescriptorSet,
    screen_size: vk::Extent2D,
}

impl Raytracing {
    pub fn new(
        device: &Device,
        screen_size: vk::Extent2D,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) -> Self {
        let output_image = Raytracing::create_storage_image(
            device,
            screen_size,
            vk::Format::B8G8R8A8_UNORM, // Matches the swapchain image format
        );
        let accumulation_image =
            Raytracing::create_storage_image(device, screen_size, vk::Format::R32G32B32A32_SFLOAT);

        let pipeline = crate::Pipeline::new(
            device,
            crate::PipelineDesc::builder()
                .raygen_path("utopian/shaders/pathtrace_reference/reference.rgen")
                .miss_path("utopian/shaders/pathtrace_reference/reference.rmiss")
                .hit_path("utopian/shaders/pathtrace_reference/reference.rchit")
                .build(),
            bindless_descriptor_set_layout,
        );

        let binding = pipeline.reflection.get_binding("topLevelAS");

        let descriptor_set = DescriptorSet::new(
            device,
            pipeline.descriptor_set_layouts[binding.set as usize],
            pipeline.reflection.get_set_mappings(binding.set),
        );

        descriptor_set.write_storage_image(
            device,
            DescriptorIdentifier::Name("output_image".to_string()),
            &output_image,
        );
        descriptor_set.write_storage_image(
            device,
            DescriptorIdentifier::Name("accumulation_image".to_string()),
            &accumulation_image,
        );

        Raytracing {
            top_level_acceleration: None,
            bottom_level_accelerations: vec![],
            output_image,
            _accumulation_image: accumulation_image,
            pipeline,
            descriptor_set,
            screen_size,
        }
    }

    pub fn initialize(&mut self, device: &Device, instances: &[ModelInstance]) {
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
            instances,
        );

        self.descriptor_set.write_acceleration_structure(
            device,
            DescriptorIdentifier::Name("topLevelAS".to_string()),
            tlas.handle,
        );

        self.top_level_acceleration = Some(tlas);
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
        let blas_buffer = Buffer::new::<u8>(
            device,
            None,
            build_sizes.acceleration_structure_size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR,
            gpu_allocator::MemoryLocation::GpuOnly,
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

        let scratch_buffer = Buffer::new::<u8>(
            device,
            None,
            build_sizes.build_scratch_size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::STORAGE_BUFFER,
            gpu_allocator::MemoryLocation::GpuOnly,
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

    pub fn fill_instance_array(
        device: &Device,
        blas: &[vk::AccelerationStructureKHR],
        instances: &[ModelInstance],
    ) -> Vec<vk::AccelerationStructureInstanceKHR> {
        let mut acceleration_instances: Vec<vk::AccelerationStructureInstanceKHR> = vec![];
        let mut blas_idx = 0;

        for instance in instances {
            for (i, mesh) in instance.model.meshes.iter().enumerate() {
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
                    instance_custom_index_and_mask: vk::Packed24_8::new(mesh.gpu_mesh, 0xff),
                    instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                        0,
                        vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as u8,
                    ),
                });

                blas_idx += 1;
            }
        }

        acceleration_instances
    }

    pub fn create_top_level_acceleration_structure(
        device: &Device,
        blas: &[vk::AccelerationStructureKHR],
        instances: &[ModelInstance],
    ) -> Tlas {
        let acceleration_instances = Self::fill_instance_array(device, blas, instances);

        let instances_buffer = Buffer::new(
            device,
            Some(acceleration_instances.as_slice()),
            std::mem::size_of_val(&*acceleration_instances) as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            gpu_allocator::MemoryLocation::GpuOnly,
        );

        let staging_instances_buffer = Buffer::create_buffer(
            device,
            std::mem::size_of_val(&*acceleration_instances) as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            gpu_allocator::MemoryLocation::CpuToGpu,
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
        let tlas_buffer = Buffer::new::<u8>(
            device,
            None,
            build_sizes.acceleration_structure_size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR,
            gpu_allocator::MemoryLocation::GpuOnly,
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

        let scratch_buffer = Buffer::new::<u8>(
            device,
            None,
            build_sizes.build_scratch_size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::STORAGE_BUFFER,
            gpu_allocator::MemoryLocation::GpuOnly,
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

        Tlas {
            handle: acceleration_structure,
            instances_buffer,
            scratch_buffer,
            staging_instances_buffer,
        }
    }

    pub fn rebuild_tlas(
        &mut self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        instances: &[ModelInstance],
    ) {
        puffin::profile_function!();

        let acceleration_instances =
            Self::fill_instance_array(device, &self.bottom_level_accelerations, instances);

        let tlas = self.top_level_acceleration.as_mut().unwrap();

        tlas.staging_instances_buffer
            .update_memory(device, acceleration_instances.as_slice());

        tlas.staging_instances_buffer.copy_to_buffer(
            device,
            command_buffer,
            &tlas.instances_buffer,
        );

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .flags(vk::GeometryFlagsKHR::OPAQUE)
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: vk::AccelerationStructureGeometryInstancesDataKHR::builder()
                    .array_of_pointers(false)
                    .data(vk::DeviceOrHostAddressConstKHR {
                        device_address: tlas.instances_buffer.get_device_address(device),
                    })
                    .build(),
            })
            .build();

        let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(ash::vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .geometries(std::slice::from_ref(&geometry))
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .dst_acceleration_structure(tlas.handle)
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: tlas.scratch_buffer.get_device_address(device),
            })
            .build();

        let build_range_info = vec![ash::vk::AccelerationStructureBuildRangeInfoKHR::builder()
            .primitive_count(acceleration_instances.len() as u32)
            .build()];

        unsafe {
            device
                .acceleration_structure_ext
                .cmd_build_acceleration_structures(
                    command_buffer,
                    std::slice::from_ref(&build_geometry_info),
                    std::slice::from_ref(&build_range_info.as_slice()),
                );
        }
    }

    pub fn create_storage_image(
        device: &Device,
        screen_size: vk::Extent2D,
        format: vk::Format,
    ) -> Image {
        let storage_image = Image::new_from_desc(
            device,
            ImageDesc::new_2d(screen_size.width, screen_size.height, format)
                .usage(vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::STORAGE),
        );

        device.execute_and_submit(|device, cb| {
            storage_image.transition_layout(device, cb, vk::ImageLayout::GENERAL);
        });

        storage_image
    }

    pub fn record_commands(
        &self,
        device: &Device,
        cb: vk::CommandBuffer,
        bindless_descriptor_set: vk::DescriptorSet,
        view_descriptor_set: vk::DescriptorSet,
        present_image: &Image,
    ) {
        unsafe {
            device.handle.cmd_bind_pipeline(
                cb,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.pipeline.handle,
            );

            // Todo: the bindless and view descriptor sets should be bound by the render graph graph
            device.handle.cmd_bind_descriptor_sets(
                cb,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.pipeline.pipeline_layout,
                DESCRIPTOR_SET_INDEX_BINDLESS,
                &[bindless_descriptor_set],
                &[],
            );

            device.handle.cmd_bind_descriptor_sets(
                cb,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.pipeline.pipeline_layout,
                crate::DESCRIPTOR_SET_INDEX_VIEW,
                &[view_descriptor_set],
                &[],
            );

            device.handle.cmd_bind_descriptor_sets(
                cb,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.pipeline.pipeline_layout,
                self.descriptor_set.get_set_index(),
                &[self.descriptor_set.handle],
                &[],
            );

            if let Some(raytracing_sbt) = &self.pipeline.raytracing_sbt {
                device.raytracing_pipeline_ext.cmd_trace_rays(
                    cb,
                    &raytracing_sbt.raygen_sbt,
                    &raytracing_sbt.miss_sbt,
                    &raytracing_sbt.hit_sbt,
                    &raytracing_sbt.callable_sbt,
                    self.screen_size.width,
                    self.screen_size.height,
                    1,
                );
            } else {
                panic!("No raytracing SBT found");
            }

            present_image.transition_layout(device, cb, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
            self.output_image
                .transition_layout(device, cb, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);

            self.output_image.copy(device, cb, present_image);

            present_image.transition_layout(device, cb, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
            self.output_image
                .transition_layout(device, cb, vk::ImageLayout::GENERAL);
        }
    }

    pub fn recreate_pipeline(
        &mut self,
        device: &Device,
        bindless_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    ) {
        self.pipeline
            .recreate_pipeline(device, bindless_descriptor_set_layout);
    }
}
