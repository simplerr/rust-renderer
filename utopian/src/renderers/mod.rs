use ash::vk;

pub mod gbuffer;
pub mod forward;

pub fn setup_render_graph(
    device: &crate::Device,
    base: &crate::VulkanBase,
    renderer: &crate::Renderer,
    camera_uniform_buffer: &crate::Buffer,
) -> crate::Graph {
    let mut graph = crate::Graph::new(&device, camera_uniform_buffer);

    let width = base.surface_resolution.width;
    let height = base.surface_resolution.height;
    let gbuffer_position = graph.create_texture(&base.device, width, height, vk::Format::R32G32B32A32_SFLOAT);
    let gbuffer_normal = graph.create_texture(&base.device, width, height, vk::Format::R32G32B32A32_SFLOAT);
    let gbuffer_albedo = graph.create_texture(&base.device, width, height, vk::Format::R8G8B8A8_UNORM);

    crate::renderers::gbuffer::setup_gbuffer_pass(
        &device,
        &mut graph,
        &renderer,
        base.depth_image,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_albedo,
    );

    crate::renderers::forward::setup_forward_pass(
        &device,
        &mut graph,
        &base,
        &renderer,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_albedo,
    );

    graph
}
