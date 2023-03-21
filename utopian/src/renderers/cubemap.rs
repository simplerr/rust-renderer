use glam::{Mat4, Vec3};

pub fn setup_cubemap_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    cubemap: crate::TextureId,
    enabled: bool,
) {
    let pipeline_handle = graph.create_pipeline(crate::PipelineDesc {
        vertex_path: "utopian/shaders/common/fullscreen.vert",
        fragment_path: "utopian/shaders/cubemap/cubemap.frag",
        vertex_input_binding_descriptions: vec![],
        vertex_input_attribute_descriptions: vec![],
        color_attachment_formats: vec![graph.resources.textures[cubemap].texture.image.format()],
        depth_stencil_attachment_format: base.depth_image.format(),
    });

    let projection = Mat4::perspective_rh(90.0_f32.to_radians(), 1.0, 0.01, 20000.0);
    let view_matrices = [
        Mat4::look_at_rh(Vec3::ZERO, Vec3::X, -Vec3::Y),
        Mat4::look_at_rh(Vec3::ZERO, -Vec3::X, -Vec3::Y),
        Mat4::look_at_rh(Vec3::ZERO, -Vec3::Y, -Vec3::Z),
        Mat4::look_at_rh(Vec3::ZERO, Vec3::Y, Vec3::Z),
        Mat4::look_at_rh(Vec3::ZERO, Vec3::Z, -Vec3::Y),
        Mat4::look_at_rh(Vec3::ZERO, -Vec3::Z, -Vec3::Y),
    ];

    for layer in 0..6 {
        graph
            .add_pass(format!("cubemap_pass{layer}"), pipeline_handle)
            .write_layer(cubemap, layer)
            .uniforms("params", &(view_matrices[layer as usize], projection))
            .render(
                move |device, command_buffer, _renderer, _pass, _resources| unsafe {
                    // Todo: This is a hack to get around the fact that we can't properly disable a pass
                    if enabled {
                        device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
                    }
                },
            )
            .build(&device, graph);
    }
}
