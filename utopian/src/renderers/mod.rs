use ash::vk;

use crate::{image::ImageDesc, TextureId};

pub mod atmosphere;
pub mod deferred;
pub mod forward;
pub mod gbuffer;
pub mod ibl;
pub mod marching_cubes;
pub mod present;
pub mod rt_reflections;
pub mod rt_shadows;
pub mod shadow;
pub mod ssao;

pub fn create_gbuffer_textures(
    graph: &mut crate::Graph,
    device: &crate::Device,
    width: u32,
    height: u32,
) -> (TextureId, TextureId, TextureId, TextureId) {
    (
        graph.create_texture(
            "gbuffer_position",
            device,
            ImageDesc::new_2d(width, height, vk::Format::R32G32B32A32_SFLOAT),
        ),
        graph.create_texture(
            "gbuffer_normal",
            device,
            ImageDesc::new_2d(width, height, vk::Format::R32G32B32A32_SFLOAT),
        ),
        graph.create_texture(
            "gbuffer_albedo",
            device,
            ImageDesc::new_2d(width, height, vk::Format::R8G8B8A8_UNORM),
        ),
        graph.create_texture(
            "gbuffer_pbr",
            device,
            ImageDesc::new_2d(width, height, vk::Format::R32G32B32A32_SFLOAT),
        ),
    )
}

pub fn create_shadowmap_texture(graph: &mut crate::Graph, device: &crate::Device) -> TextureId {
    graph.create_texture(
        "shadow_map",
        device,
        ImageDesc::new_2d_array(4096, 4096, 4, vk::Format::D32_SFLOAT)
            .aspect(vk::ImageAspectFlags::DEPTH)
            .usage(
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::SAMPLED,
            ),
    )
}

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

    let (gbuffer_position, gbuffer_normal, gbuffer_albedo, gbuffer_pbr) =
        create_gbuffer_textures(graph, device, width, height);

    let shadow_map = create_shadowmap_texture(graph, device);

    let deferred_output = graph.create_texture(
        "deferred_output",
        device,
        ImageDesc::new_2d(width, height, vk::Format::R32G32B32A32_SFLOAT),
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

    let rt_shadows = crate::renderers::rt_shadows::setup_rt_shadows_pass(
        device,
        graph,
        gbuffer_position,
        gbuffer_normal,
        width,
        height,
        view_data.raytracing_supported == 1,
    );

    crate::renderers::gbuffer::setup_gbuffer_pass(
        device,
        graph,
        base,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_albedo,
        gbuffer_pbr,
    );

    let (environment_map, irradiance_map, specular_map, brdf_lut) =
        crate::renderers::ibl::setup_cubemap_pass(device, graph, renderer);

    let rt_reflections = crate::renderers::rt_reflections::setup_rt_reflections_pass(
        device,
        graph,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_pbr,
        irradiance_map,
        specular_map,
        brdf_lut,
        width,
        height,
        view_data.raytracing_supported == 1,
    );

    crate::renderers::ssao::setup_ssao_pass(
        device,
        graph,
        gbuffer_position,
        gbuffer_normal,
        ssao_output,
        view_data.ssao_enabled == 1,
    );

    crate::renderers::deferred::setup_deferred_pass(
        device,
        graph,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_albedo,
        gbuffer_pbr,
        shadow_map,
        rt_shadows,
        rt_reflections,
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
            base,
            deferred_output,
            shadow_map,
            (cascade_matrices, cascade_depths),
            true,
        );
    }

    crate::renderers::atmosphere::setup_atmosphere_pass(
        device,
        graph,
        base,
        deferred_output,
        environment_map,
        camera,
        true,
    );

    crate::renderers::present::setup_present_pass(device, graph, deferred_output);
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

    let (gbuffer_position, gbuffer_normal, gbuffer_albedo, gbuffer_pbr) =
        create_gbuffer_textures(graph, device, width, height);

    struct Reservoir {
        total_weight: f32,
        sample_weight: f32,
        light_index: u32,
        M: u32,
    }

    let initial_ris_reservoirs = graph.create_buffer(
        "initial_ris_reservoirs",
        device,
        (width * height * std::mem::size_of::<Reservoir>() as u32) as u64,
        ash::vk::BufferUsageFlags::STORAGE_BUFFER,
        gpu_allocator::MemoryLocation::GpuOnly,
    );

    let spatial_reuse_reservoirs = graph.create_buffer(
        "spatial_reuse_reservoirs",
        device,
        (width * height * std::mem::size_of::<Reservoir>() as u32) as u64,
        ash::vk::BufferUsageFlags::STORAGE_BUFFER,
        gpu_allocator::MemoryLocation::GpuOnly,
    );

    let temporal_reuse_reservoirs = graph.create_buffer(
        "temporal_reuse_reservoirs",
        device,
        (width * height * std::mem::size_of::<Reservoir>() as u32) as u64,
        ash::vk::BufferUsageFlags::STORAGE_BUFFER,
        gpu_allocator::MemoryLocation::GpuOnly,
    );

    crate::renderers::gbuffer::setup_gbuffer_pass(
        device,
        graph,
        base,
        gbuffer_position,
        gbuffer_normal,
        gbuffer_albedo,
        gbuffer_pbr,
    );

    graph
        .add_pass_from_desc(
            "reset_reservoirs_pass",
            crate::PipelineDesc::builder()
                .compute_path("utopian/shaders/restir/reset_reservoirs.comp"),
        )
        .write_buffer(initial_ris_reservoirs)
        .write_buffer(spatial_reuse_reservoirs)
        .write_buffer(temporal_reuse_reservoirs)
        .dispatch((width + 16 - 1) / 16, (height + 16 - 1) / 16, 1)
        .build(device, graph);

    graph
        .add_pass_from_desc(
            "initial_ris_pass",
            crate::PipelineDesc::builder()
                .raygen_path("utopian/shaders/restir/initial_ris.rgen")
                .miss_path("utopian/shaders/restir/initial_ris.rmiss")
                .hit_path("utopian/shaders/restir/initial_ris.rchit"),
        )
        .tlas(0)
        .read(gbuffer_position)
        .write_buffer(initial_ris_reservoirs)
        .trace_rays(width, height, 1)
        .build(device, graph);

    graph
        .add_pass_from_desc(
            "temporal_reuse_pass",
            crate::PipelineDesc::builder()
                .raygen_path("utopian/shaders/restir/temporal_reuse.rgen")
                .miss_path("utopian/shaders/restir/temporal_reuse.rmiss")
                .hit_path("utopian/shaders/restir/temporal_reuse.rchit"),
        )
        .tlas(0)
        .read(gbuffer_position)
        .read(gbuffer_normal)
        .read_buffer(initial_ris_reservoirs)
        .read_buffer(spatial_reuse_reservoirs) // prev_frame_reservoirs
        .write_buffer(temporal_reuse_reservoirs)
        .trace_rays(width, height, 1)
        .build(device, graph);

    graph
        .add_pass_from_desc(
            "spatial_reuse_pass",
            crate::PipelineDesc::builder()
                .raygen_path("utopian/shaders/restir/spatial_reuse.rgen")
                .miss_path("utopian/shaders/restir/spatial_reuse.rmiss")
                .hit_path("utopian/shaders/restir/spatial_reuse.rchit"),
        )
        .tlas(0)
        .read(gbuffer_position)
        .read_buffer(temporal_reuse_reservoirs)
        .write_buffer(spatial_reuse_reservoirs)
        .trace_rays(width, height, 1)
        .build(device, graph);

    // It is possible to do the spatial resampling recursively to reduce noise
    // graph
    //     .add_pass_from_desc(
    //         "spatial_reuse_pass_2",
    //         crate::PipelineDesc::builder()
    //             .raygen_path("utopian/shaders/restir/spatial_reuse.rgen")
    //             .miss_path("utopian/shaders/restir/spatial_reuse.rmiss")
    //             .hit_path("utopian/shaders/restir/spatial_reuse.rchit"),
    //     )
    //     .tlas(0)
    //     .read(gbuffer_position)
    //     .read_buffer(spatial_reuse_reservoirs)
    //     .write_buffer(initial_ris_reservoirs)
    //     .trace_rays(width, height, 1)
    //     .build(device, graph);

    // graph
    //     .add_pass_from_desc(
    //         "spatial_reuse_pass_3",
    //         crate::PipelineDesc::builder()
    //             .raygen_path("utopian/shaders/restir/spatial_reuse.rgen")
    //             .miss_path("utopian/shaders/restir/spatial_reuse.rmiss")
    //             .hit_path("utopian/shaders/restir/spatial_reuse.rchit"),
    //     )
    //     .tlas(0)
    //     .read(gbuffer_position)
    //     .read_buffer(initial_ris_reservoirs)
    //     .write_buffer(spatial_reuse_reservoirs)
    //     .trace_rays(width, height, 1)
    //     .build(device, graph);

    graph
        .add_pass_from_desc(
            "reference_pt_pass",
            crate::PipelineDesc::builder()
                .raygen_path("utopian/shaders/pathtrace_reference/reference.rgen")
                .miss_path("utopian/shaders/pathtrace_reference/reference.rmiss")
                .hit_path("utopian/shaders/pathtrace_reference/reference.rchit"),
        )
        .tlas(0)
        .read_buffer(spatial_reuse_reservoirs)
        .image_write(output_image)
        .image_write(accumulation_image)
        .trace_rays(width, height, 1)
        .build(device, graph);

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
        .build(device, graph);
}

pub fn build_hybrid_render_graph(
    _graph: &mut crate::Graph,
    _device: &crate::Device,
    base: &crate::VulkanBase,
    _renderer: &crate::Renderer,
    _view_data: &crate::ViewUniformData,
    _camera: &crate::Camera,
) {
    puffin::profile_function!();

    let _width = base.surface_resolution.width;
    let _height = base.surface_resolution.height;

    // Todo
}

pub fn build_minimal_forward_render_graph(
    graph: &mut crate::Graph,
    device: &crate::Device,
    base: &crate::VulkanBase,
    view_data: &crate::ViewUniformData,
    camera: &crate::Camera,
) {
    puffin::profile_function!();

    let width = base.surface_resolution.width;
    let height = base.surface_resolution.height;
    let rgba32_fmt = vk::Format::R32G32B32A32_SFLOAT;

    // Forward & deferred output textures
    let forward_output = graph.create_texture(
        "forward_output",
        device,
        ImageDesc::new_2d(width, height, rgba32_fmt),
    );
    let shadow_map = create_shadowmap_texture(graph, device);

    let (cascade_matrices, cascade_depths) = crate::renderers::shadow::setup_shadow_pass(
        device,
        graph,
        shadow_map,
        view_data.sun_dir,
        camera,
        view_data.shadows_enabled == 1,
    );

    crate::renderers::forward::setup_forward_pass(
        device,
        graph,
        base,
        forward_output,
        shadow_map,
        (cascade_matrices, cascade_depths),
    );

    crate::renderers::present::setup_present_pass(device, graph, forward_output);
}
