use ash::vk;

pub mod gbuffer;
pub mod pbr;

pub fn setup_render_graph(
    device: &crate::Device,
    base: &crate::VulkanBase,
    renderer: &crate::Renderer,
    camera_uniform_buffer: &crate::Buffer,
) -> crate::Graph {
    let mut graph = crate::Graph::new(&device, camera_uniform_buffer);

    let gbuffer_position = graph.create_texture(&base.device, 800, 600, vk::Format::R8G8B8A8_UNORM);
    let gbuffer_normal = graph.create_texture(&base.device, 800, 600, vk::Format::R8G8B8A8_UNORM);
    let gbuffer_albedo = graph.create_texture(&base.device, 800, 600, vk::Format::R8G8B8A8_UNORM);

    crate::renderers::gbuffer::setup_gbuffer_pass(
        &device,
        &mut graph,
        &renderer,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_albedo,
    );

    crate::renderers::pbr::setup_pbr_pass(
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
