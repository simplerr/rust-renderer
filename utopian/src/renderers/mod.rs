pub mod pbr;
pub mod test;

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

    let colored_rect_texture = graph.create_texture(&base.device, 800, 600);

    crate::renderers::test::setup_test_pass(&device, &mut graph, &renderer, colored_rect_texture);

    crate::renderers::pbr::setup_pbr_pass(
        &device,
        &mut graph,
        &base,
        &renderer,
        colored_rect_texture,
        &camera_uniform_buffer,
    );

    graph
}
