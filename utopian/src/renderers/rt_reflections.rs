use crate::image::ImageDesc;
use ash::vk;

pub fn setup_rt_reflections_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    gbuffer_position: crate::TextureId,
    gbuffer_normal: crate::TextureId,
    gbuffer_pbr: crate::TextureId,
    width: u32,
    height: u32,
) -> crate::TextureId {
    puffin::profile_function!();

    let output_image = graph.create_texture(
        "rt_reflection_output_image",
        device,
        ImageDesc::new_2d(width, height, vk::Format::R8G8B8A8_UNORM),
    );

    graph
        .add_pass_from_desc(
            "rt_reflections_pass",
            crate::PipelineDesc::builder()
                .raygen_path("utopian/shaders/rt_reflections/rt_reflections.rgen")
                .miss_path("utopian/shaders/rt_reflections/rt_reflections.rmiss")
                .hit_path("utopian/shaders/rt_reflections/rt_reflections.rchit"),
        )
        .tlas(0)
        .read(gbuffer_position)
        .read(gbuffer_normal)
        .read(gbuffer_pbr)
        .image_write(output_image)
        .trace_rays(width, height, 1)
        .build(&device, graph);

    output_image
}
