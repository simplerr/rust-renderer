use ash::vk;

#[allow(dead_code)]
struct PushConstants {
    world: glam::Mat4,
    color: glam::Vec4,
    mesh_index: u32,
    pad: [u32; 3],
}

pub fn setup_gbuffer_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    gbuffer_position: crate::TextureId,
    gbuffer_normal: crate::TextureId,
    gbuffer_albedo: crate::TextureId,
    gbuffer_pbr: crate::TextureId,
) {
    let pipeline_handle = graph.create_pipeline(crate::PipelineDesc {
        vertex_path: "utopian/shaders/gbuffer/gbuffer.vert",
        fragment_path: "utopian/shaders/gbuffer/gbuffer.frag",
        vertex_input_binding_descriptions: crate::Primitive::get_vertex_input_binding_descriptions(
        ),
        vertex_input_attribute_descriptions:
            crate::Primitive::get_vertex_input_attribute_descriptions(),
        color_attachment_formats: vec![
            graph.resources.textures[gbuffer_position]
                .texture
                .image
                .format(),
            graph.resources.textures[gbuffer_normal]
                .texture
                .image
                .format(),
            graph.resources.textures[gbuffer_albedo]
                .texture
                .image
                .format(),
            graph.resources.textures[gbuffer_pbr].texture.image.format(),
        ],
        depth_stencil_attachment_format: vk::Format::D32_SFLOAT,
    });

    // let depth_image = crate::Image::new(
    //     device,
    //     graph.resources[gbuffer_position].texture.image.width,
    //     graph.resources[gbuffer_position].texture.image.height,
    //     vk::Format::D32_SFLOAT,
    //     vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
    //     vk::ImageAspectFlags::DEPTH,
    // );

    graph
        .add_pass(String::from("gbuffer_pass"), pipeline_handle)
        .write(gbuffer_position)
        .write(gbuffer_normal)
        .write(gbuffer_albedo)
        .write(gbuffer_pbr)
        //.depth_attachment(depth_image)
        .external_depth_attachment(base.depth_image.clone(), vk::AttachmentLoadOp::CLEAR) // Todo: create own Depth image
        .render(move |device, command_buffer, renderer, pass, resources| {
            let pipeline = resources.pipeline(pass.pipeline_handle);

            renderer.draw_meshes(device, command_buffer, pipeline.pipeline_layout);
        })
        .build(&device, graph);
}
