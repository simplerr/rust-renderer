use ash::vk;

use crate::image::ImageDesc;

pub mod deferred;
pub mod forward;
pub mod gbuffer;
pub mod present;
pub mod shadow;

pub fn build_render_graph(
    graph: &mut crate::Graph,
    device: &crate::Device,
    base: &crate::VulkanBase,
    _renderer: &crate::Renderer,
    sun_dir: glam::Vec3, // Todo: pass more than just sun direction
) {
    puffin::profile_function!();

    //let mut graph = crate::Graph::new(&device, camera_uniform_buffer);

    let extent = [
        base.surface_resolution.width,
        base.surface_resolution.height,
    ];
    let rgba8_format = vk::Format::R8G8B8A8_UNORM;
    let rgba32_fmt = vk::Format::R32G32B32A32_SFLOAT;

    // Todo: cache textures

    // G-buffer textures
    let gbuffer_position = graph.create_texture(
        "gbuffer_position",
        device,
        ImageDesc::new_2d(extent[0], extent[1], rgba32_fmt),
    );

    let gbuffer_normal = graph.create_texture(
        "gbuffer_normal",
        device,
        ImageDesc::new_2d(extent[0], extent[1], rgba32_fmt),
    );
    let gbuffer_albedo = graph.create_texture(
        "gbuffer_albedo",
        device,
        ImageDesc::new_2d(extent[0], extent[1], rgba8_format),
    );
    let gbuffer_pbr = graph.create_texture(
        "gbuffer_pbr",
        device,
        ImageDesc::new_2d(extent[0], extent[1], rgba32_fmt),
    );

    // Shadow map
    let shadow_map = graph.create_texture(
        "shadow_map",
        device,
        ImageDesc::new_2d_array(4096, 4096, 4, vk::Format::D32_SFLOAT)
            .aspect(vk::ImageAspectFlags::DEPTH)
            .usage(
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::SAMPLED,
            ),
    );

    // Forward & deferred output textures
    let forward_output = graph.create_texture(
        "forward_output",
        device,
        ImageDesc::new_2d(extent[0], extent[1], rgba32_fmt),
    );
    let deferred_output = graph.create_texture(
        "deferred_output",
        device,
        ImageDesc::new_2d(extent[0], extent[1], rgba32_fmt),
    );

    let cascade_matrices = crate::renderers::shadow::setup_shadow_pass(device, graph, base, shadow_map, sun_dir);

    crate::renderers::gbuffer::setup_gbuffer_pass(
        &device,
        graph,
        &base,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_albedo,
        gbuffer_pbr,
    );

    crate::renderers::forward::setup_forward_pass(
        &device,
        graph,
        &base,
        forward_output,
        shadow_map,
        cascade_matrices,
    );

    crate::renderers::deferred::setup_deferred_pass(
        &device,
        graph,
        &base,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_albedo,
        gbuffer_pbr,
        deferred_output,
    );

    crate::renderers::present::setup_present_pass(
        &device,
        graph,
        &base,
        forward_output,
        deferred_output,
        shadow_map,
    );

    // let forward_renderer = crate::renderers::forward::ForwardRenderer::new(
    //     &device,
    //     graph,
    //     &base,
    //     &renderer,
    //     forward_output,
    // );

    // forward_renderer.render(device, graph, base, renderer, forward_output);

    //graph
}
