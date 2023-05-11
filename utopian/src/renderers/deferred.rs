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
    gbuffer_position: crate::TextureId,
    gbuffer_normal: crate::TextureId,
    gbuffer_albedo: crate::TextureId,
    gbuffer_pbr: crate::TextureId,
    shadow_map: crate::TextureId,
    rt_shadows: crate::TextureId,
    ssao_output: crate::TextureId,
    irradiance_map: crate::TextureId,
    specular_map: crate::TextureId,
    brdf_lut: crate::TextureId,
    cascade_data: ([glam::Mat4; 4], [f32; 4]),
    deferred_output: crate::TextureId,
) {
    puffin::profile_function!();

    graph
        .add_pass_from_desc(
            "deferred_pass",
            crate::PipelineDesc::builder()
                .vertex_path("utopian/shaders/common/fullscreen.vert")
                .fragment_path("utopian/shaders/deferred/deferred.frag"),
        )
        .read(gbuffer_position)
        .read(gbuffer_normal)
        .read(gbuffer_albedo)
        .read(gbuffer_pbr)
        .read(shadow_map)
        .read(rt_shadows)
        .read(ssao_output)
        .read(irradiance_map)
        .read(specular_map)
        .read(brdf_lut)
        .write(deferred_output)
        .uniforms("shadowmapParams", &(cascade_data))
        .render(
            move |device, command_buffer, _renderer, _pass, _resources| unsafe {
                device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
            },
        )
        .build(&device, graph);
}
