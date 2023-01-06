use ash::vk;

#[allow(dead_code)]
struct PushConstants {
    world: glam::Mat4,
    color: glam::Vec4,
    mesh_index: u32,
    pad: [u32; 3],
}

pub fn setup_shadow_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    shadow_map: crate::TextureId,
) {
    puffin::profile_function!();

    let pipeline_handle = graph.create_pipeline(crate::PipelineDesc {
        vertex_path: "utopian/shaders/shadow/shadow.vert",
        fragment_path: "utopian/shaders/shadow/shadow.frag",
        vertex_input_binding_descriptions: crate::Primitive::get_vertex_input_binding_descriptions(
        ),
        vertex_input_attribute_descriptions:
            crate::Primitive::get_vertex_input_attribute_descriptions(),
        color_attachment_formats: vec![],
        depth_stencil_attachment_format: base.depth_image.format(),
    });

    let cascade_directions = [
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 0.0, 1.0),
        glam::Vec3::new(-1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 0.0, -1.0),
    ];

    let num_cascades = 4;
    for cascade in 0..num_cascades {
        let view_matrix = glam::Mat4::look_at_rh(
            glam::Vec3::new(0.0, 0.0, 0.0),
            cascade_directions[cascade],
            glam::Vec3::new(0.0, 1.0, 0.0),
        );

        let projection_matrix = glam::Mat4::perspective_rh(
            f32::to_radians(60.0),
            2000.0 / 1100.0,
            0.01,
            20000.0,
        );

        let view_projection_matrix = projection_matrix * view_matrix;
        graph
            .add_pass(format!("shadow_pass_{cascade}"), pipeline_handle)
            // Todo: only one uniform buffer with this name is created!
            .uniforms("cascade_view_projection", &view_projection_matrix)
            .depth_attachment_layer(shadow_map, cascade as u32)
            .render(
                move |device, command_buffer, renderer, pass, resources| {
                    let pipeline = resources.pipeline(pass.pipeline_handle);

                    renderer.draw_meshes(device, command_buffer, pipeline.pipeline_layout);
                },
            )
            .build(&device, graph);
    }
}
