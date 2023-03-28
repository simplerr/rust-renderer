use ash::vk;
use glam::{Mat4, Vec3};

use crate::image::ImageDesc;

pub fn setup_cubemap_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    renderer: &crate::Renderer,
    enabled: bool,
) -> (
    crate::TextureId,
    crate::TextureId,
    crate::TextureId,
    crate::TextureId,
) {
    let (mip0_size, num_mips) = (256, 5);

    // Todo: can use smaller format?
    let rgba32_fmt = vk::Format::R32G32B32A32_SFLOAT;

    let environment_map = graph.create_texture(
        "environment_map",
        device,
        ImageDesc::new_cubemap(256, 256, rgba32_fmt).mip_levels(5),
    );

    let irradiance_map = graph.create_texture(
        "irradiance_map",
        device,
        ImageDesc::new_cubemap(256, 256, rgba32_fmt), //.mip_levels(5),
    );

    let specular_map = graph.create_texture(
        "specular_map",
        device,
        ImageDesc::new_cubemap(256, 256, rgba32_fmt).mip_levels(5),
    );

    let offscreen = graph.create_texture(
        "cubemap_offscreen",
        device,
        ImageDesc::new_2d(mip0_size, mip0_size, rgba32_fmt),
    );

    let brdf_lut = graph.create_texture(
        "brdf_lut",
        device,
        ImageDesc::new_2d(512, 512, vk::Format::R16G16_SFLOAT),
    );

    let cubemap_pipeline = graph.create_pipeline(crate::PipelineDesc {
        vertex_path: "utopian/shaders/common/fullscreen.vert",
        fragment_path: "utopian/shaders/ibl/cubemap.frag",
        vertex_input_binding_descriptions: vec![],
        vertex_input_attribute_descriptions: vec![],
        color_attachment_formats: vec![graph.resources.textures[environment_map]
            .texture
            .image
            .format()],
        depth_stencil_attachment_format: base.depth_image.format(),
    });

    let irradiance_filter_pipeline = graph.create_pipeline(crate::PipelineDesc {
        vertex_path: "utopian/shaders/common/fullscreen.vert",
        fragment_path: "utopian/shaders/ibl/irradiance_filter.frag",
        vertex_input_binding_descriptions: vec![],
        vertex_input_attribute_descriptions: vec![],
        color_attachment_formats: vec![graph.resources.textures[irradiance_map]
            .texture
            .image
            .format()],
        depth_stencil_attachment_format: base.depth_image.format(), // Todo: skip this if depth is not needed
    });

    let specular_filter_pipeline = graph.create_pipeline(crate::PipelineDesc {
        vertex_path: "utopian/shaders/ibl/fullscreen_with_pushconst.vert",
        fragment_path: "utopian/shaders/ibl/specular_filter.frag",
        vertex_input_binding_descriptions: vec![],
        vertex_input_attribute_descriptions: vec![],
        color_attachment_formats: vec![graph.resources.textures[specular_map]
            .texture
            .image
            .format()],
        depth_stencil_attachment_format: base.depth_image.format(), // Todo: skip this if depth is not needed
    });

    let brdf_lut_pipeline = graph.create_pipeline(crate::PipelineDesc {
        vertex_path: "utopian/shaders/common/fullscreen.vert",
        fragment_path: "utopian/shaders/ibl/brdf_lut.frag",
        vertex_input_binding_descriptions: vec![],
        vertex_input_attribute_descriptions: vec![],
        color_attachment_formats: vec![graph.resources.textures[brdf_lut].texture.image.format()],
        depth_stencil_attachment_format: base.depth_image.format(),
    });

    let projection = Mat4::perspective_rh(90.0_f32.to_radians(), 1.0, 0.01, 20000.0);
    let view_matrices = [
        Mat4::look_at_rh(Vec3::ZERO, Vec3::X, -Vec3::Y),
        Mat4::look_at_rh(Vec3::ZERO, -Vec3::X, -Vec3::Y),
        Mat4::look_at_rh(Vec3::ZERO, -Vec3::Y, -Vec3::Z),
        Mat4::look_at_rh(Vec3::ZERO, Vec3::Y, Vec3::Z),
        Mat4::look_at_rh(Vec3::ZERO, Vec3::Z, -Vec3::Y),
        Mat4::look_at_rh(Vec3::ZERO, -Vec3::Z, -Vec3::Y),
    ];

    for mip in 0..num_mips {
        let mip_size = mip0_size as f32 * 0.5f32.powf(mip as f32);

        for layer in 0..6 {
            graph
                .add_pass(
                    format!("cubemap_pass_layer_{layer}_mip_{mip}"),
                    cubemap_pipeline,
                )
                .write(offscreen)
                .uniforms("params", &(view_matrices[layer as usize], projection))
                .render(move |device, cb, _renderer, _pass, _resources| unsafe {
                    // Todo: This is a hack to get around the fact that we can't properly disable a pass
                    if enabled {
                        let viewports = [vk::Viewport {
                            x: 0.0,
                            y: mip_size as f32,
                            width: mip_size as f32,
                            height: -(mip_size as f32),
                            min_depth: 0.0,
                            max_depth: 1.0,
                        }];

                        device.handle.cmd_set_viewport(cb, 0, &viewports);
                        device.handle.cmd_draw(cb, 3, 1, 0, 0);
                    }
                })
                .copy_image(
                    offscreen,
                    environment_map,
                    vk::ImageCopy::builder()
                        .src_subresource(
                            vk::ImageSubresourceLayers::builder()
                                .mip_level(0)
                                .base_array_layer(0)
                                .layer_count(1)
                                .build(),
                        )
                        .dst_subresource(
                            vk::ImageSubresourceLayers::builder()
                                .mip_level(mip)
                                .base_array_layer(layer)
                                .layer_count(1)
                                .build(),
                        )
                        .extent(vk::Extent3D {
                            width: mip_size as u32,
                            height: mip_size as u32,
                            depth: 1,
                        })
                        .build(),
                )
                .build(&device, graph);
        }
    }

    // Irradiance filter pass (mip 0 only)
    for layer in 0..6 {
        graph
            .add_pass(
                format!("irradiance_filter_pass_layer_{layer}"),
                irradiance_filter_pipeline,
            )
            .active(renderer.need_environment_map_update)
            .read(environment_map)
            .write_layer(irradiance_map, layer)
            .uniforms("params", &(view_matrices[layer as usize], projection))
            .render(move |device, cb, _renderer, _pass, _resources| unsafe {
                // Todo: This is a hack to get around the fact that we can't properly disable a pass
                if enabled {
                    // Todo: make helper
                    let viewports = [vk::Viewport {
                        x: 0.0,
                        y: mip0_size as f32,
                        width: mip0_size as f32,
                        height: -(mip0_size as f32),
                        min_depth: 0.0,
                        max_depth: 1.0,
                    }];

                    device.handle.cmd_set_viewport(cb, 0, &viewports);
                    device.handle.cmd_draw(cb, 3, 1, 0, 0);
                }
            })
            .build(&device, graph);
    }

    // Specular filter pass (all mip levels)
    for mip in 0..num_mips {
        let mip_size = mip0_size as f32 * 0.5f32.powf(mip as f32);

        for layer in 0..6 {
            graph
                .add_pass(
                    format!("specular_filter_pass_layer_{layer}_mip_{mip}"),
                    specular_filter_pipeline,
                )
                .active(renderer.need_environment_map_update)
                .read(environment_map)
                .write(offscreen)
                .uniforms("params", &(view_matrices[layer as usize], projection))
                .render(move |device, cb, _renderer, pass, resources| unsafe {
                    // Todo: This is a hack to get around the fact that we can't properly disable a pass
                    if enabled {
                        let viewports = [vk::Viewport {
                            x: 0.0,
                            y: mip_size as f32,
                            width: mip_size as f32,
                            height: -(mip_size as f32),
                            min_depth: 0.0,
                            max_depth: 1.0,
                        }];

                        device.handle.cmd_set_viewport(cb, 0, &viewports);

                        let roughness = mip as f32 / (num_mips - 1) as f32;

                        let pipeline = resources.pipeline(pass.pipeline_handle);
                        device.handle.cmd_push_constants(
                            cb,
                            pipeline.pipeline_layout,
                            vk::ShaderStageFlags::ALL,
                            0,
                            std::slice::from_raw_parts(
                                &roughness as *const _ as *const u8,
                                std::mem::size_of_val(&roughness),
                            ),
                        );

                        device.handle.cmd_draw(cb, 3, 1, 0, 0);
                    }
                })
                .copy_image(
                    offscreen,
                    specular_map,
                    vk::ImageCopy::builder()
                        .src_subresource(
                            vk::ImageSubresourceLayers::builder()
                                .mip_level(0)
                                .base_array_layer(0)
                                .layer_count(1)
                                .build(),
                        )
                        .dst_subresource(
                            vk::ImageSubresourceLayers::builder()
                                .mip_level(mip)
                                .base_array_layer(layer)
                                .layer_count(1)
                                .build(),
                        )
                        .extent(vk::Extent3D {
                            width: mip_size as u32,
                            height: mip_size as u32,
                            depth: 1,
                        })
                        .build(),
                )
                .build(&device, graph);
        }
    }

    graph
        .add_pass(String::from("brdf_lut_pass"), brdf_lut_pipeline)
        .active(renderer.need_environment_map_update)
        .write(brdf_lut)
        .render(
            move |device, command_buffer, _renderer, _pass, _resources| unsafe {
                device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
            },
        )
        .build(&device, graph);

    (environment_map, irradiance_map, specular_map, brdf_lut)
}