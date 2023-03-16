use ash::vk;
use glam::{Mat4, Vec3};

pub fn setup_atmosphere_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    atmosphere_output: crate::TextureId,
    camera: &crate::camera::Camera,
    enabled: bool,
) {
    let pipeline_handle = graph.create_pipeline(crate::PipelineDesc {
        vertex_path: "utopian/shaders/atmosphere/atmosphere.vert",
        fragment_path: "utopian/shaders/atmosphere/atmosphere.frag",
        vertex_input_binding_descriptions: crate::Primitive::get_vertex_input_binding_descriptions(
        ),
        vertex_input_attribute_descriptions:
            crate::Primitive::get_vertex_input_attribute_descriptions(),
        color_attachment_formats: vec![graph.resources.textures[atmosphere_output]
            .texture
            .image
            .format()],
        depth_stencil_attachment_format: base.depth_image.format(),
    });

    let projection = camera.get_projection();
    // Mat4::from_rotation_x(std::f32::consts::PI) * 
    let world = Mat4::from_scale(Vec3::splat(16.0));

    graph
        .add_pass(String::from("atmosphere_pass"), pipeline_handle)
        .write(atmosphere_output)
        .uniforms("ubo_constants", &(projection, world))
        .render(
            move |device, command_buffer, _renderer, _pass, _resources| unsafe {
                // Todo: This is a hack to get around the fact that we can't properly disable a pass
                if enabled {
                    device.handle.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[_renderer.instances[0].model.meshes[0]
                            .primitive
                            .vertex_buffer
                            .buffer],
                        &[0],
                    );
                    device.handle.cmd_bind_index_buffer(
                        command_buffer,
                        _renderer.instances[0].model.meshes[0]
                            .primitive
                            .index_buffer
                            .buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    device.handle.cmd_draw_indexed(
                        command_buffer,
                        _renderer.instances[0].model.meshes[0]
                            .primitive
                            .indices
                            .len() as u32,
                        1,
                        0,
                        0,
                        1,
                    );
                }
            },
        )
        .build(&device, graph);
}
