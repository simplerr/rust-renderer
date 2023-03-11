pub fn setup_present_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    forward_output: crate::TextureId,
    deferred_output: crate::TextureId,
    shadow_map: crate::TextureId,
) {
    puffin::profile_function!();

    let pipeline_handle = graph.create_pipeline(crate::PipelineDesc {
        vertex_path: "utopian/shaders/common/fullscreen.vert",
        fragment_path: "utopian/shaders/present/present.frag",
        vertex_input_binding_descriptions: vec![],
        vertex_input_attribute_descriptions: vec![],
        color_attachment_formats: vec![base.present_images[0].format()],
        depth_stencil_attachment_format: base.depth_image.format(),
    });

    graph
        .add_pass(String::from("present_pass"), pipeline_handle)
        .read(forward_output)
        .read(deferred_output)
        .read(shadow_map)
        .presentation_pass(true)
        .external_depth_attachment(base.depth_image.clone())
        .render(
            move |device, command_buffer, _renderer, _pass, _pipeline_cache| unsafe {
                device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
            },
        )
        .build(&device, graph);
}
