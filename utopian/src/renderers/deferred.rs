use ash::vk;

#[allow(dead_code)]
struct PushConstants {
    world: glam::Mat4,
    color: glam::Vec4,
    mesh_index: u32,
    pad: [u32; 3],
}

pub fn setup_deferred_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    renderer: &crate::Renderer,
    gbuffer_position: crate::TextureId,
    gbuffer_normal: crate::TextureId,
    gbuffer_albedo: crate::TextureId,
    gbuffer_pbr: crate::TextureId,
    deferred_output: crate::TextureId,
) {
    let pipeline = crate::Pipeline::new(
        &device.handle,
        crate::PipelineDesc {
            vertex_path: "utopian/shaders/common/fullscreen.vert",
            fragment_path: "utopian/shaders/deferred/deferred.frag",
            vertex_input_binding_descriptions: vec![],
            vertex_input_attribute_descriptions: vec![],
            color_attachment_formats: vec![graph.resources[deferred_output].texture.image.format],
            depth_stencil_attachment_format: base.depth_image.format,
        },
        Some(renderer.bindless_descriptor_set_layout),
    );

    // Data & buffer setup
    let color = glam::Vec3::new(0.0, 0.0, 1.0);
    let slice = unsafe { std::slice::from_raw_parts(&color, 1) };
    let test_uniform_buffer = crate::Buffer::new(
        &base.device,
        Some(slice),
        std::mem::size_of_val(&color) as u64,
        vk::BufferUsageFlags::UNIFORM_BUFFER,
        gpu_allocator::MemoryLocation::CpuToGpu,
    );

    // Descriptor set setup
    let test_binding = pipeline.reflection.get_binding("test_params");
    let test_descriptor_set = crate::DescriptorSet::new(
        device,
        pipeline.descriptor_set_layouts[test_binding.set as usize],
        pipeline.reflection.get_set_mappings(test_binding.set),
    );
    test_descriptor_set.write_uniform_buffer(
        device,
        "test_params".to_string(),
        &test_uniform_buffer,
    );

    graph
        .add_pass(String::from("deferred_pass"), pipeline)
        .read(gbuffer_position)
        .read(gbuffer_normal)
        .read(gbuffer_albedo)
        .read(gbuffer_pbr)
        .write(deferred_output)
        .depth_attachment(base.depth_image)
        .render(move |device, command_buffer, renderer, pass| unsafe {
            // Todo: move to common place
            device.handle.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pass.pipeline.pipeline_layout,
                crate::DESCRIPTOR_SET_INDEX_BINDLESS,
                &[renderer.bindless_descriptor_set],
                &[],
            );

            device.handle.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pass.pipeline.pipeline_layout,
                test_binding.set,
                &[test_descriptor_set.handle],
                &[],
            );

            device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
        })
        .build(&device);
}
