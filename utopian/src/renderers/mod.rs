use ash::vk;

pub mod deferred;
pub mod forward;
pub mod gbuffer;
pub mod present;

pub fn setup_render_graph(
    device: &crate::Device,
    base: &crate::VulkanBase,
    renderer: &crate::Renderer,
    camera_uniform_buffer: &crate::Buffer,
) -> crate::Graph {
    puffin::profile_function!();

    let mut graph = crate::Graph::new(&device, camera_uniform_buffer);

    let extent = [
        base.surface_resolution.width,
        base.surface_resolution.height,
    ];
    let rgba8_format = vk::Format::R8G8B8A8_UNORM;
    let rgba32_fmt = vk::Format::R32G32B32A32_SFLOAT;

    // G-buffer textures
    let gbuffer_position = graph.create_texture("gbuffer_position", device, extent, rgba32_fmt);
    let gbuffer_normal = graph.create_texture("gbuffer_normal", device, extent, rgba32_fmt);
    let gbuffer_albedo = graph.create_texture("gbuffer_albedo", device, extent, rgba8_format);
    let gbuffer_pbr = graph.create_texture("gbuffer_pbr", device, extent, rgba32_fmt);

    // Forward & deferred output textures
    let forward_output = graph.create_texture("forward_output", device, extent, rgba32_fmt);
    let deferred_output = graph.create_texture("deferred_output", device, extent, rgba32_fmt);

    crate::renderers::gbuffer::setup_gbuffer_pass(
        &device,
        &mut graph,
        &renderer,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_albedo,
        gbuffer_pbr,
    );

    crate::renderers::forward::setup_forward_pass(
        &device,
        &mut graph,
        &base,
        &renderer,
        forward_output,
    );

    crate::renderers::deferred::setup_deferred_pass(
        &device,
        &mut graph,
        &base,
        &renderer,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_albedo,
        gbuffer_pbr,
        deferred_output,
    );

    crate::renderers::present::setup_present_pass(
        &device,
        &mut graph,
        &base,
        &renderer,
        forward_output,
        deferred_output,
    );

    graph
}
