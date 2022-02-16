use ash::vk;
use glam::Vec3;
use std::mem;

use crate::buffer::*;
use crate::device::*;

pub struct Raytracing {}

impl Raytracing {
    pub fn create_bottom_acceleration_structure(device: &Device) -> vk::AccelerationStructureKHR {
        let indices = vec![0, 1, 2];

        let vertices = vec![
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(-1.0, 1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
        ];

        // let transform = vk::TransformMatrixKHR {
        //     matrix: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        // };

        let index_buffer = Buffer::new(
            device,
            indices.as_slice(),
            std::mem::size_of_val(&*indices) as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
        );

        let vertex_buffer = Buffer::new(
            device,
            vertices.as_slice(),
            std::mem::size_of_val(&*vertices) as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
        );

        // let transform_buffer = Buffer::new(
        //     device,
        //     transform.as_slice(),
        //     std::mem::size_of_val(&*transform) as u64,
        //     vk::BufferUsageFlags::VERTEX_BUFFER,
        // );

        let vertex_buffer_device_address = vk::DeviceOrHostAddressConstKHR {
            device_address: vertex_buffer.get_device_address(device),
        };
        let index_buffer_device_address = vk::DeviceOrHostAddressConstKHR {
            device_address: index_buffer.get_device_address(device),
        };

        let geometries = vk::AccelerationStructureGeometryKHR::builder()
            .flags(vk::GeometryFlagsKHR::OPAQUE)
            .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                triangles: vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
                    .vertex_format(vk::Format::R32G32B32_SFLOAT)
                    .vertex_data(vertex_buffer_device_address)
                    .vertex_stride(mem::size_of::<Vec3>() as _)
                    .max_vertex(3)
                    .index_type(vk::IndexType::UINT32)
                    .index_data(index_buffer_device_address)
                    .build(),
            })
            .build();

        // First used to get the build sizes
        let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(ash::vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .geometries(std::slice::from_ref(&geometries))
            .build();

        let num_triangles = 1;

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
            .geometries(std::slice::from_ref(&geometries))
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

        println!("Created bottom level acceleration structure");
        println!("{:#?}", build_sizes);

        acceleration_structure
    }
}
