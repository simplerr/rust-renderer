pub mod gbuffer;
pub mod pbr;

pub fn setup_render_graph(
    device: &crate::Device,
    base: &crate::VulkanBase,
    renderer: &crate::Renderer,
    camera_uniform_buffer: &crate::Buffer,
) -> crate::Graph {
    let mut graph = crate::Graph {
        passes: vec![],
        resources: vec![],
    };

    let gbuffer_position = graph.create_texture(&base.device, 800, 600);
    let gbuffer_normal = graph.create_texture(&base.device, 800, 600);
    let gbuffer_albedo = graph.create_texture(&base.device, 800, 600);

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
        &camera_uniform_buffer,
    );

    graph
}
