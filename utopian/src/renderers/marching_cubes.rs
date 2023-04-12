use std::num;

use glam::Vec4;

use crate::{PipelineDesc, Vertex};

pub fn setup_marching_cubes_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    deferred_output: crate::TextureId,
    enabled: bool,
) {
    puffin::profile_function!();

    let num_voxels = 32 as u32;
    let size = num_voxels.pow(3) * std::mem::size_of::<Vertex>() as u32;

    let voxel_size: f32 = 10.0;
    let view_distance: f32 = 100.0;
    let flat_normals: bool = false;

    let offsets = [
        Vec4::new(0.0, 0.0, 0.0, 0.0),
        Vec4::new(voxel_size, 0.0, 0.0, 0.0),
        Vec4::new(voxel_size, voxel_size, 0.0, 0.0),
        Vec4::new(0.0, voxel_size, 0.0, 0.0),
        Vec4::new(0.0, 0.0, voxel_size, 0.0),
        Vec4::new(voxel_size, 0.0, voxel_size, 0.0),
        Vec4::new(voxel_size, voxel_size, voxel_size, 0.0),
        Vec4::new(0.0, voxel_size, voxel_size, 0.0),
    ];

    let color = Vec4::new(0.0, 1.0, 0.0, 1.0);

    let data_tuple = (
        offsets,
        color,
        voxel_size,
        view_distance,
        num_voxels,
        flat_normals,
    );

    let vertex_buffer = graph.create_buffer(
        "marching_cubes_vertex_buffer",
        device,
        size as u64,
        ash::vk::BufferUsageFlags::STORAGE_BUFFER | ash::vk::BufferUsageFlags::VERTEX_BUFFER,
        gpu_allocator::MemoryLocation::GpuOnly,
    );

    let counter_buffer = graph.create_buffer(
        "marching_cubes_counter_buffer",
        device,
        4,
        ash::vk::BufferUsageFlags::STORAGE_BUFFER,
        gpu_allocator::MemoryLocation::GpuOnly,
    );

    graph
        .add_pass_from_desc(
            "marching_cubes_pass",
            PipelineDesc::builder()
                .compute_path("utopian/shaders/marching_cubes/marching_cubes.comp"),
        )
        .write_buffer(counter_buffer)
        .write_buffer(vertex_buffer)
        .uniforms("ubo", &data_tuple)
        .dispatch(num_voxels, num_voxels, num_voxels)
        .build(&device, graph);
}
