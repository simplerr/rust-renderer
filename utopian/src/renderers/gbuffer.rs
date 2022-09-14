use ash::vk;

pub fn setup_gbuffer_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    renderer: &crate::Renderer,
    gbuffer_position: crate::TextureId,
    gbuffer_normal: crate::TextureId,
    gbuffer_albedo: crate::TextureId,
) {
    let pipeline = crate::Pipeline::new(
        &device.handle,
        crate::PipelineDesc {
            vertex_path: "utopian/shaders/common/fullscreen.vert",
            fragment_path: "utopian/shaders/gbuffer.frag",
            vertex_input_binding_descriptions: vec![],
            vertex_input_attribute_descriptions: vec![],
        },
        &[
            graph.resources[gbuffer_position].texture.image.format,
            graph.resources[gbuffer_normal].texture.image.format,
            graph.resources[gbuffer_albedo].texture.image.format,
        ],
        vk::Format::D32_SFLOAT,
        Some(renderer.bindless_descriptor_set_layout),
    );

    graph
        .add_pass(String::from("gbuffer_pass"), pipeline)
        .write(gbuffer_position)
        .write(gbuffer_normal)
        .write(gbuffer_albedo)
        .render(move |device, command_buffer, _renderer, _pass| unsafe {
            device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
        })
        .build(&device);
}
