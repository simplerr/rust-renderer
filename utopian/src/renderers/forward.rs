use std::sync::Arc;

use ash::vk;

#[allow(dead_code)]
struct PushConstants {
    world: glam::Mat4,
    color: glam::Vec4,
    mesh_index: u32,
    pad: [u32; 3],
}

pub fn setup_forward_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    renderer: &crate::Renderer,
    forward_output: crate::TextureId,
    shadow_map: crate::TextureId,
    cascade_matrices: [glam::Mat4; 4],
) {
    puffin::profile_function!();

    let pipeline_handle = graph.create_pipeline(crate::PipelineDesc {
        vertex_path: "utopian/shaders/forward/forward.vert",
        fragment_path: "utopian/shaders/forward/forward.frag",
        vertex_input_binding_descriptions: crate::Primitive::get_vertex_input_binding_descriptions(
        ),
        vertex_input_attribute_descriptions:
            crate::Primitive::get_vertex_input_attribute_descriptions(),
        color_attachment_formats: vec![graph.resources.textures[forward_output]
            .texture
            .image
            .format()],
        depth_stencil_attachment_format: base.depth_image.format(),
    });

    graph
        .add_pass(String::from("forward_pass"), pipeline_handle)
        .read(shadow_map)
        .write(forward_output)
        .uniforms("shadowmapParams", &(cascade_matrices))
        .external_depth_attachment(base.depth_image.clone()) // Todo: create own Depth image
        .render(
            move |device, command_buffer, renderer, pass, resources| {
                let pipeline = resources.pipeline(pass.pipeline_handle);

                renderer.draw_meshes(device, command_buffer, pipeline.pipeline_layout);
            },
        )
        .build(&device, graph);
}
