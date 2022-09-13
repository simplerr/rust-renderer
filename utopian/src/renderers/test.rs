use ash::vk;

pub fn setup_test_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    renderer: &crate::Renderer,
    colored_rect_texture: &crate::Texture,
) {
    let test_pipeline = crate::Pipeline::new(
        &device.handle,
        crate::PipelineDesc {
            vertex_path: "utopian/shaders/common/fullscreen.vert",
            fragment_path: "utopian/shaders/colored_rect.frag",
            vertex_input_binding_descriptions: vec![],
            vertex_input_attribute_descriptions: vec![],
        },
        &[colored_rect_texture.image.format],
        vk::Format::D32_SFLOAT,
        Some(renderer.bindless_descriptor_set_layout),
    );

    graph
        .add_pass(String::from("test_pass"), test_pipeline)
        .write(
            crate::GraphResourceId::ColoredRectTexture,
            colored_rect_texture.image,
        )
        .render(move |device, command_buffer, _renderer, _pass| unsafe {
            device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
        })
        .build();
}
