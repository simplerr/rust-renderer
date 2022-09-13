use ash::vk;

#[allow(dead_code)]
struct PushConstants {
    world: glam::Mat4,
    color: glam::Vec4,
    mesh_index: u32,
    pad: [u32; 3],
}

pub fn setup_pbr_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    renderer: &crate::Renderer,
    colored_rect_texture: &crate::Texture,
    camera_uniform_buffer: &crate::Buffer,
) {
    let pipeline = crate::Pipeline::new(
        &device.handle,
        crate::PipelineDesc {
            vertex_path: "prototype/shaders/pbr/pbr.vert",
            fragment_path: "prototype/shaders/pbr/pbr.frag",
            vertex_input_binding_descriptions:
                crate::Primitive::get_vertex_input_binding_descriptions(),
            vertex_input_attribute_descriptions:
                crate::Primitive::get_vertex_input_attribute_descriptions(),
        },
        &[base.present_images[0].format],
        base.depth_image.format,
        Some(renderer.bindless_descriptor_set_layout),
    );

    let camera_binding = pipeline.reflection.get_binding("camera");

    let descriptor_set_camera = crate::DescriptorSet::new(
        &base.device,
        pipeline.descriptor_set_layouts[camera_binding.set as usize],
        pipeline.reflection.get_set_mappings(camera_binding.set),
    );

    descriptor_set_camera.write_uniform_buffer(
        &base.device,
        "camera".to_string(),
        &camera_uniform_buffer,
    );

    // Todo: this should be moved to the render graph some way
    descriptor_set_camera.write_combined_image(
        &base.device,
        "inputTexture".to_string(),
        &colored_rect_texture,
    );

    graph
        .add_pass(String::from("pbr_pass"), pipeline)
        .read(
            crate::GraphResourceId::ColoredRectTexture,
            colored_rect_texture.image,
        )
        .presentation_pass(true)
        .depth_attachment(base.depth_image)
        .render(move |device, command_buffer, renderer, pass| unsafe {
            device.handle.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pass.pipeline.pipeline_layout,
                camera_binding.set,
                &[descriptor_set_camera.handle],
                &[],
            );

            device.handle.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pass.pipeline.pipeline_layout,
                crate::BINDLESS_DESCRIPTOR_INDEX,
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
                        pass.pipeline.pipeline_layout,
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
        })
        .build();
}
