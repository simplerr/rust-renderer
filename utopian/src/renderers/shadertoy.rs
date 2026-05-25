#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct ShadertoyInputs {
    i_resolution: glam::Vec3,
    i_time: f32,
    i_mouse: glam::Vec4,
    i_date: glam::Vec4,
    i_time_delta: f32,
    i_frame: i32,
    _pad: [f32; 2],
}

pub fn setup_shadertoy_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    shader_path: &'static str,
    shadertoy_output: crate::TextureId,
    time: f32,
) {
    puffin::profile_function!();

    let width = base.surface_resolution.width as f32;
    let height = base.surface_resolution.height as f32;

    let inputs = ShadertoyInputs {
        i_resolution: glam::Vec3::new(width, height, 1.0),
        i_time: time,
        i_mouse: glam::Vec4::ZERO,
        i_date: glam::Vec4::ZERO,
        i_time_delta: 1.0 / 60.0,
        i_frame: 0,
        _pad: [0.0, 0.0],
    };

    graph
        .add_pass_from_desc(
            "shadertoy_pass",
            crate::PipelineDesc::builder()
                .vertex_path("utopian/shaders/common/fullscreen.vert")
                .fragment_path(shader_path),
        )
        .write(shadertoy_output)
        .uniforms("shadertoy_inputs", &inputs)
        .render(
            move |device, command_buffer, _renderer, _pass, _resources| unsafe {
                device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
            },
        )
        .build(device, graph);
}
