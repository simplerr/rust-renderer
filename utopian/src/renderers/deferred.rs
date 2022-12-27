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
    let pipeline_handle = graph.create_pipeline(crate::PipelineDesc {
        vertex_path: "utopian/shaders/common/fullscreen.vert",
        fragment_path: "utopian/shaders/deferred/deferred.frag",
        vertex_input_binding_descriptions: vec![],
        vertex_input_attribute_descriptions: vec![],
        color_attachment_formats: vec![graph.resources[deferred_output].texture.image.format],
        depth_stencil_attachment_format: base.depth_image.format,
    });

    graph
        .add_pass(String::from("deferred_pass"), pipeline_handle)
        .read(gbuffer_position)
        .read(gbuffer_normal)
        .read(gbuffer_albedo)
        .read(gbuffer_pbr)
        .write(deferred_output)
        .uniforms("test_params_2", &(glam::Vec3::new(1.0, 0.0, 0.0)))
        .depth_attachment(base.depth_image)
        .render(
            move |device, command_buffer, renderer, pass, pipeline_cache| unsafe {
                let pipeline = &pipeline_cache[pass.pipeline_handle];

                // Todo: move to common place
                device.handle.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.pipeline_layout,
                    crate::DESCRIPTOR_SET_INDEX_BINDLESS,
                    &[renderer.bindless_descriptor_set],
                    &[],
                );

                device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
            },
        )
        .build(&device, graph);
}
