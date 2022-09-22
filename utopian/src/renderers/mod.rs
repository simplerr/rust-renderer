use ash::vk;

pub mod forward;
pub mod gbuffer;
pub mod deferred;

pub fn setup_render_graph(
    device: &crate::Device,
    base: &crate::VulkanBase,
    renderer: &crate::Renderer,
    camera_uniform_buffer: &crate::Buffer,
) -> crate::Graph {
    let mut graph = crate::Graph::new(&device, camera_uniform_buffer);

    let width = base.surface_resolution.width;
    let height = base.surface_resolution.height;
    let gbuffer_position = graph.create_texture(
        "gbuffer_position",
        &base.device,
        width,
        height,
        vk::Format::R32G32B32A32_SFLOAT,
    );
    let gbuffer_normal = graph.create_texture(
        "gbuffer_normal",
        &base.device,
        width,
        height,
        vk::Format::R32G32B32A32_SFLOAT,
    );
    let gbuffer_albedo = graph.create_texture(
        "gbuffer_albedo",
        &base.device,
        width,
        height,
        vk::Format::R8G8B8A8_UNORM,
    );
    let gbuffer_pbr = graph.create_texture(
        "gbuffer_pbr",
        &base.device,
        width,
        height,
        vk::Format::R32G32B32A32_SFLOAT,
    );

    crate::renderers::gbuffer::setup_gbuffer_pass(
        &device,
        &mut graph,
        &renderer,
        base.depth_image,
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
    );

    // crate::renderers::deferred::setup_deferred_pass(
    //     &device,
    //     &mut graph,
    //     &base,
    //     &renderer,
    //     gbuffer_position,
    //     gbuffer_normal,
    //     gbuffer_albedo,
    //     gbuffer_pbr,
    // );

    graph
}
