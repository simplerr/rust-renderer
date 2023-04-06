use crate::PipelineDesc;

pub fn setup_present_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    forward_output: crate::TextureId,
    deferred_output: crate::TextureId,
    shadow_map: crate::TextureId,
) {
    puffin::profile_function!();

    let workgroup_size_x = 16;
    let workgroup_size_y = 16;
    let image_size_x = 512;
    let image_size_y = 512;
    let num_workgroups_x = (image_size_x + workgroup_size_x - 1) / workgroup_size_x;
    let num_workgroups_y = (image_size_y + workgroup_size_y - 1) / workgroup_size_y;

    let fxaa_threshold = 0.45;

    let test_buffer = graph.create_buffer(
        "test_buffer",
        device,
        512 * 4,
        ash::vk::BufferUsageFlags::STORAGE_BUFFER,
        gpu_allocator::MemoryLocation::GpuOnly,
    );

    graph
        .add_pass_from_desc(
            "compute_pass_write_buffer",
            PipelineDesc::builder().compute_path("utopian/shaders/compute_test_write_buffer.comp"),
        )
        .write_buffer(test_buffer)
        .dispatch(num_workgroups_x, 1, 1)
        .build(&device, graph);

    graph
        .add_pass_from_desc(
            "compute_pass",
            PipelineDesc::builder().compute_path("utopian/shaders/compute_test.comp"),
        )
        .read(forward_output)
        .image_write(deferred_output)
        .read_buffer(test_buffer)
        .dispatch(num_workgroups_x, num_workgroups_y, 1)
        .build(&device, graph);

    graph
        .add_pass_from_desc(
            "present_pass",
            PipelineDesc::builder()
                .vertex_path("utopian/shaders/common/fullscreen.vert")
                .fragment_path("utopian/shaders/present/present.frag"),
        )
        .read(forward_output)
        .read(deferred_output)
        .read(shadow_map)
        .read_buffer(test_buffer)
        .uniforms(
            "settings_fxaa",
            &(glam::Vec4::new(1.0, 0.0, fxaa_threshold, 0.0)),
        )
        .presentation_pass(true)
        .render(
            move |device, command_buffer, _renderer, _pass, _pipeline_cache| unsafe {
                device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
            },
        )
        .build(&device, graph);
}
