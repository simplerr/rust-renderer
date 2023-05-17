use ash::vk;
use glam::Vec4;

use crate::{PipelineDesc, Vertex};

pub fn setup_marching_cubes_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    deferred_output: crate::TextureId,
    shadow_map: crate::TextureId,
    cascade_data: ([glam::Mat4; 4], [f32; 4]),
    _enabled: bool,
) {
    puffin::profile_function!();

    let num_voxels = 32_u32;
    let mut size = num_voxels.pow(3) * std::mem::size_of::<Vertex>() as u32;
    size *= 5;

    let voxel_size: f32 = 1.0;
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

    let draw_command_buffer = graph.create_buffer(
        "marching_cubes_draw_command_buffer",
        device,
        std::mem::size_of::<vk::DrawIndirectCommand>() as u64,
        ash::vk::BufferUsageFlags::STORAGE_BUFFER | ash::vk::BufferUsageFlags::INDIRECT_BUFFER,
        gpu_allocator::MemoryLocation::GpuOnly,
    );

    graph
        .add_pass_from_desc(
            "marching_cubes_reset_counter_pass",
            PipelineDesc::builder()
                .compute_path("utopian/shaders/marching_cubes/reset_counter.comp"),
        )
        .write_buffer(draw_command_buffer)
        .dispatch(1, 1, 1)
        .build(device, graph);

    graph
        .add_pass_from_desc(
            "marching_cubes_pass",
            PipelineDesc::builder()
                .compute_path("utopian/shaders/marching_cubes/marching_cubes.comp"),
        )
        .write_buffer(draw_command_buffer)
        .write_buffer(vertex_buffer)
        .uniforms("ubo", &data_tuple)
        .dispatch(num_voxels, num_voxels, num_voxels)
        .build(device, graph);

    graph
        .add_pass_from_desc(
            "marching_cubes_forward_pass",
            crate::PipelineDesc::builder()
                // Reusing the forward shader is not ideal since this mesh is not part of the bindless data
                .vertex_path("utopian/shaders/forward/forward.vert")
                .fragment_path("utopian/shaders/forward/forward.frag")
                .default_primitive_vertex_bindings()
                .default_primitive_vertex_attributes(),
        )
        .read(shadow_map)
        .load_write(deferred_output)
        .external_depth_attachment(base.depth_image.clone(), vk::AttachmentLoadOp::LOAD)
        .extra_barriers(&[
            (draw_command_buffer, vk_sync::AccessType::IndirectBuffer),
            (vertex_buffer, vk_sync::AccessType::VertexBuffer),
        ])
        .uniforms("shadowmapParams", &(cascade_data))
        .render(
            move |device, command_buffer, _renderer, pass, resources| unsafe {
                let pipeline = resources.pipeline(pass.pipeline_handle);

                device.cmd_push_constants(
                    command_buffer,
                    pipeline.pipeline_layout,
                    (
                        glam::Mat4::IDENTITY,
                        glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
                        0, // Mesh index not used
                        [0; 3],
                    ),
                );

                device.handle.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[resources.buffer(vertex_buffer).buffer.buffer],
                    &[0],
                );

                let stride = std::mem::size_of::<vk::DrawIndirectCommand>() as u32;
                device.handle.cmd_draw_indirect(
                    command_buffer,
                    resources.buffer(draw_command_buffer).buffer.buffer,
                    0,
                    1,
                    stride,
                );
            },
        )
        .build(device, graph);
}
