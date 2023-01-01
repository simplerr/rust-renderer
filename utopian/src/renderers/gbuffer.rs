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
    renderer: &crate::Renderer,
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
            graph.resources.textures[gbuffer_position].texture.image.format,
            graph.resources.textures[gbuffer_normal].texture.image.format,
            graph.resources.textures[gbuffer_albedo].texture.image.format,
            graph.resources.textures[gbuffer_pbr].texture.image.format,
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
        .depth_attachment(base.depth_image) // Todo: create own Depth image
        .render(
            move |device, command_buffer, renderer, pass, resources| unsafe {
                let pipeline = resources.pipeline(pass.pipeline_handle);

                // Todo: move to common place
                device.handle.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.pipeline_layout,
                    crate::DESCRIPTOR_SET_INDEX_BINDLESS,
                    &[renderer.bindless_descriptor_set],
                    &[],
                );

                for instance in &renderer.instances {
                    for (i, mesh) in instance.model.meshes.iter().enumerate() {
                        let push_data = PushConstants {
                            world: instance.transform * instance.model.transforms[i],
                            color: glam::Vec4::new(1.0, 0.5, 0.2, 1.0),
                            mesh_index: mesh.gpu_mesh,
                            pad: [0; 3],
                        };

                        device.handle.cmd_push_constants(
                            command_buffer,
                            pipeline.pipeline_layout,
                            vk::ShaderStageFlags::ALL,
                            0,
                            std::slice::from_raw_parts(
                                &push_data as *const _ as *const u8,
                                std::mem::size_of_val(&push_data),
                            ),
                        );

                        device.handle.cmd_bind_vertex_buffers(
                            command_buffer,
                            0,
                            &[mesh.primitive.vertex_buffer.buffer],
                            &[0],
                        );
                        device.handle.cmd_bind_index_buffer(
                            command_buffer,
                            mesh.primitive.index_buffer.buffer,
                            0,
                            vk::IndexType::UINT32,
                        );
                        device.handle.cmd_draw_indexed(
                            command_buffer,
                            mesh.primitive.indices.len() as u32,
                            1,
                            0,
                            0,
                            1,
                        );
                    }
                }
            },
        )
        .build(&device, graph);
}
