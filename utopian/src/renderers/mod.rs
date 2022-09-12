pub mod pbr;
pub mod test;

pub fn setup_render_graph(
    device: &crate::Device,
    base: &crate::VulkanBase,
    renderer: &crate::Renderer,
    camera_uniform_buffer: &crate::Buffer,
) -> crate::Graph {
    let colored_rect_texture = crate::Texture::create(&base.device, None, 800, 600);

    let mut graph = crate::Graph {
        passes: vec![],
        resources: std::collections::HashMap::new(),
    };

    crate::renderers::test::setup_test_pass(&device, &mut graph, &renderer, &colored_rect_texture);

    crate::renderers::pbr::setup_pbr_pass(
        &device,
        &mut graph,
        &base,
        &renderer,
        &colored_rect_texture,
        &camera_uniform_buffer,
    );

    graph
}
