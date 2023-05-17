use crate::image::ImageDesc;
use ash::vk;

pub fn setup_rt_shadows_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    gbuffer_position: crate::TextureId,
    gbuffer_normal: crate::TextureId,
    width: u32,
    height: u32,
    enabled: bool,
) -> crate::TextureId {
    puffin::profile_function!();

    let output_image = graph.create_texture(
        "rt_shadows_output_image",
        device,
        ImageDesc::new_2d(width, height, vk::Format::R8_UNORM),
    );

    if enabled {
        graph
            .add_pass_from_desc(
                "rt_shadows_pass",
                crate::PipelineDesc::builder()
                    .raygen_path("utopian/shaders/rt_shadows/rt_shadows.rgen")
                    .miss_path("utopian/shaders/rt_shadows/rt_shadows.rmiss")
                    .hit_path("utopian/shaders/rt_shadows/rt_shadows.rchit"),
            )
            .tlas(0)
            .read(gbuffer_position)
            .read(gbuffer_normal)
            .image_write(output_image)
            .trace_rays(width, height, 1)
            .build(device, graph);
    }

    output_image
}
