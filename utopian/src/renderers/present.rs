
pub fn setup_present_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    renderer: &crate::Renderer,
    forward_output: crate::TextureId,
    deferred_output: crate::TextureId,
) {
    let pipeline = crate::Pipeline::new(
        &device.handle,
        crate::PipelineDesc {
            vertex_path: "utopian/shaders/common/fullscreen.vert",
            fragment_path: "utopian/shaders/present/present.frag",
            vertex_input_binding_descriptions: vec![],
            vertex_input_attribute_descriptions: vec![],
            color_attachment_formats: vec![base.present_images[0].format],
            depth_stencil_attachment_format: base.depth_image.format,
        },
        Some(renderer.bindless_descriptor_set_layout),
    );

    graph
        .add_pass(String::from("present_pass"), pipeline)
        .read(forward_output)
        .read(deferred_output)
        .presentation_pass(true)
        .depth_attachment(base.depth_image)
        .render(move |device, command_buffer, _renderer, _pass| unsafe {
            device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
        })
        .build(&device);
}