pub fn setup_present_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    forward_output: crate::TextureId,
    deferred_output: crate::TextureId,
    shadow_map: crate::TextureId,
) {
    puffin::profile_function!();

    let pipeline_handle = graph.create_pipeline(
        crate::PipelineDesc::builder()
            .vertex_path("utopian/shaders/common/fullscreen.vert")
            .fragment_path("utopian/shaders/present/present.frag")
            .build(),
    );

    let fxaa_threshold = 0.45;

    graph
        .add_pass(String::from("present_pass"), pipeline_handle)
        .read(forward_output)
        .read(deferred_output)
        .read(shadow_map)
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
