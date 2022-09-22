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
) {
    let pipeline = crate::Pipeline::new(
        &device.handle,
        crate::PipelineDesc {
            vertex_path: "utopian/shaders/forward/forward.vert",
            fragment_path: "utopian/shaders/forward/forward.frag",
            vertex_input_binding_descriptions:
                crate::Primitive::get_vertex_input_binding_descriptions(),
            vertex_input_attribute_descriptions:
                crate::Primitive::get_vertex_input_attribute_descriptions(),
            color_attachment_formats: vec![graph.resources[forward_output].texture.image.format],
            // Todo:
            depth_stencil_attachment_format: base.depth_image.format,
        },
        Some(renderer.bindless_descriptor_set_layout),
    );

    // Data & buffer setup
    let color = glam::Vec3::new(0.0, 0.0, 1.0);
    let slice = unsafe { std::slice::from_raw_parts(&color, 1) };
    let test_uniform_buffer = crate::Buffer::new(
        &base.device,
        Some(slice),
        std::mem::size_of_val(&color) as u64,
        vk::BufferUsageFlags::UNIFORM_BUFFER,
        gpu_allocator::MemoryLocation::CpuToGpu,
    );

    // Descriptor set setup
    let test_binding = pipeline.reflection.get_binding("test_params");
    let test_descriptor_set = crate::DescriptorSet::new(
        device,
        pipeline.descriptor_set_layouts[test_binding.set as usize],
        pipeline.reflection.get_set_mappings(test_binding.set),
    );
    test_descriptor_set.write_uniform_buffer(
        device,
        "test_params".to_string(),
        &test_uniform_buffer,
    );

    graph
        .add_pass(String::from("forward_pass"), pipeline)
        .write(forward_output)
        .depth_attachment(base.depth_image)
        .render(move |device, command_buffer, renderer, pass| unsafe {
            // Todo: move to common place
            device.handle.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pass.pipeline.pipeline_layout,
                crate::DESCRIPTOR_SET_INDEX_BINDLESS,
                &[renderer.bindless_descriptor_set],
                &[],
            );

            device.handle.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pass.pipeline.pipeline_layout,
                test_binding.set,
                &[test_descriptor_set.handle],
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
        .build(&device);
}
