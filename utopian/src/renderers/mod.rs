use ash::vk;

use crate::image::ImageDesc;

pub mod atmosphere;
pub mod deferred;
pub mod forward;
pub mod gbuffer;
pub mod ibl;
pub mod marching_cubes;
pub mod present;
pub mod shadow;
pub mod ssao;

pub fn build_render_graph(
    graph: &mut crate::Graph,
    device: &crate::Device,
    base: &crate::VulkanBase,
    renderer: &crate::Renderer,
    view_data: &crate::ViewUniformData,
    camera: &crate::Camera,
) {
    puffin::profile_function!();

    let width = base.surface_resolution.width;
    let height = base.surface_resolution.height;
    let rgba8_format = vk::Format::R8G8B8A8_UNORM;
    let rgba32_fmt = vk::Format::R32G32B32A32_SFLOAT;

    // G-buffer textures
    let gbuffer_position = graph.create_texture(
        "gbuffer_position",
        device,
        ImageDesc::new_2d(width, height, rgba32_fmt),
    );
    let gbuffer_normal = graph.create_texture(
        "gbuffer_normal",
        device,
        ImageDesc::new_2d(width, height, rgba32_fmt),
    );
    let gbuffer_albedo = graph.create_texture(
        "gbuffer_albedo",
        device,
        ImageDesc::new_2d(width, height, rgba8_format),
    );
    let gbuffer_pbr = graph.create_texture(
        "gbuffer_pbr",
        device,
        ImageDesc::new_2d(width, height, rgba32_fmt),
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
        ImageDesc::new_2d(width, height, rgba32_fmt),
    );
    let deferred_output = graph.create_texture(
        "deferred_output",
        device,
        ImageDesc::new_2d(width, height, rgba32_fmt),
    );

    let ssao_output = graph.create_texture(
        "ssao_output",
        device,
        ImageDesc::new_2d(width, height, vk::Format::R16_UNORM),
    );

    let (cascade_matrices, cascade_depths) = crate::renderers::shadow::setup_shadow_pass(
        device,
        graph,
        shadow_map,
        view_data.sun_dir,
        camera,
        view_data.shadows_enabled == 1,
    );

    crate::renderers::gbuffer::setup_gbuffer_pass(
        &device,
        graph,
        &base,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_albedo,
        gbuffer_pbr,
    );

    crate::renderers::ssao::setup_ssao_pass(
        &device,
        graph,
        gbuffer_position,
        gbuffer_normal,
        ssao_output,
        view_data.ssao_enabled == 1,
    );

    let (environment_map, irradiance_map, specular_map, brdf_lut) =
        crate::renderers::ibl::setup_cubemap_pass(&device, graph, renderer);

    crate::renderers::forward::setup_forward_pass(
        &device,
        graph,
        &base,
        forward_output,
        shadow_map,
        (cascade_matrices, cascade_depths),
    );

    crate::renderers::deferred::setup_deferred_pass(
        &device,
        graph,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_albedo,
        gbuffer_pbr,
        shadow_map,
        ssao_output,
        irradiance_map,
        specular_map,
        brdf_lut,
        (cascade_matrices, cascade_depths),
        deferred_output,
    );

    if view_data.marching_cubes_enabled == 1 {
        crate::renderers::marching_cubes::setup_marching_cubes_pass(
            device,
            graph,
            &base,
            deferred_output,
            shadow_map,
            (cascade_matrices, cascade_depths),
            true,
        );
    }

    crate::renderers::atmosphere::setup_atmosphere_pass(
        &device,
        graph,
        &base,
        deferred_output,
        environment_map,
        camera,
        true,
    );

    crate::renderers::present::setup_present_pass(
        &device,
        graph,
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

pub fn build_path_tracing_render_graph(
    graph: &mut crate::Graph,
    device: &crate::Device,
    base: &crate::VulkanBase,
) {
    puffin::profile_function!();

    let width = base.surface_resolution.width;
    let height = base.surface_resolution.height;

    let output_image = graph.create_texture(
        "pt_output_image",
        device,
        ImageDesc::new_2d(width, height, vk::Format::B8G8R8A8_UNORM),
    );

    let accumulation_image = graph.create_texture(
        "pt_accumulation_image",
        device,
        ImageDesc::new_2d(width, height, vk::Format::R32G32B32A32_SFLOAT),
    );

    graph
        .add_pass_from_desc(
            "reference_pt_pass",
            crate::PipelineDesc::builder()
                .raygen_path("utopian/shaders/raytracing_basic/basic.rgen")
                .miss_path("utopian/shaders/raytracing_basic/basic.rmiss")
                .hit_path("utopian/shaders/raytracing_basic/basic.rchit"),
        )
        .tlas(0)
        .image_write(output_image)
        .image_write(accumulation_image)
        .trace_rays(width, height, 1)
        .build(&device, graph);

    graph
        .add_pass_from_desc(
            "reference_pt_present_pass",
            crate::PipelineDesc::builder()
                .vertex_path("utopian/shaders/common/fullscreen.vert")
                .fragment_path("utopian/shaders/blit/blit.frag"),
        )
        .read(output_image)
        .presentation_pass(true)
        .render(
            move |device, command_buffer, _renderer, _pass, _resources| unsafe {
                device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
            },
        )
        .build(&device, graph);
}
