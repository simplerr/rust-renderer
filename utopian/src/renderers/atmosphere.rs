use ash::vk;
use glam::{Mat4, Vec3};

pub fn setup_atmosphere_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    atmosphere_output: crate::TextureId,
    environment_map: crate::TextureId,
    camera: &crate::camera::Camera,
    enabled: bool,
) {
    puffin::profile_function!();

    let pipeline_handle = graph.create_pipeline(
        crate::PipelineDesc::builder()
            .vertex_path("utopian/shaders/atmosphere/atmosphere.vert")
            .fragment_path("utopian/shaders/atmosphere/atmosphere.frag")
            .default_primitive_vertex_bindings()
            .default_primitive_vertex_attributes()
            .build(),
    );

    let projection = camera.get_projection();
    let world = Mat4::from_scale(Vec3::splat(1000.0));

    graph
        .add_pass(String::from("atmosphere_pass"), pipeline_handle)
        .load_write(atmosphere_output)
        .read(environment_map)
        .uniforms("ubo_constants", &(projection, world))
        .external_depth_attachment(base.depth_image.clone(), vk::AttachmentLoadOp::LOAD)
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
