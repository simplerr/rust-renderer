use ash::vk;
use glam::{Mat4, Vec3};

use crate::{image::ImageDesc, render_utils};

pub fn setup_cubemap_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    renderer: &crate::Renderer,
) -> (
    crate::TextureId,
    crate::TextureId,
    crate::TextureId,
    crate::TextureId,
) {
    puffin::profile_function!();

    let (mip0_size, num_mips) = (512, 8);

    // Todo: can use smaller format?
    let rgba32_fmt = vk::Format::R32G32B32A32_SFLOAT;

    let environment_map = graph.create_texture(
        "environment_map",
        device,
        ImageDesc::new_cubemap(mip0_size, mip0_size, rgba32_fmt).mip_levels(num_mips),
    );

    let irradiance_map = graph.create_texture(
        "irradiance_map",
        device,
        ImageDesc::new_cubemap(mip0_size, mip0_size, rgba32_fmt),
    );

    let specular_map = graph.create_texture(
        "specular_map",
        device,
        ImageDesc::new_cubemap(mip0_size, mip0_size, rgba32_fmt).mip_levels(num_mips),
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

    let projection = Mat4::perspective_rh(90.0_f32.to_radians(), 1.0, 0.01, 20000.0);
    let view_matrices = [
        Mat4::look_at_rh(Vec3::ZERO, Vec3::X, -Vec3::Y),
        Mat4::look_at_rh(Vec3::ZERO, -Vec3::X, -Vec3::Y),
        Mat4::look_at_rh(Vec3::ZERO, -Vec3::Y, -Vec3::Z),
        Mat4::look_at_rh(Vec3::ZERO, Vec3::Y, Vec3::Z),
        Mat4::look_at_rh(Vec3::ZERO, Vec3::Z, -Vec3::Y),
        Mat4::look_at_rh(Vec3::ZERO, -Vec3::Z, -Vec3::Y),
    ];

    // Do not add any passes to the graph if we don't need to update the environment map
    if !renderer.need_environment_map_update {
        return (environment_map, irradiance_map, specular_map, brdf_lut);
    }

    for mip in 0..num_mips {
        let size = (mip0_size as f32 * 0.5f32.powf(mip as f32)) as u32;

        for layer in 0..6 {
            graph
                .add_pass_from_desc(
                    format!("cubemap_pass_layer_{layer}_mip_{mip}").as_str(),
                    crate::PipelineDesc::builder()
                        .vertex_path("utopian/shaders/common/fullscreen.vert")
                        .fragment_path("utopian/shaders/ibl/cubemap.frag"),
                )
                .write(offscreen)
                .uniforms("params", &(view_matrices[layer as usize], projection))
                .render(move |device, cb, _renderer, _pass, _resources| unsafe {
                    let viewport = [render_utils::viewport(size, size)];
                    device.handle.cmd_set_viewport(cb, 0, &viewport);
                    device.handle.cmd_draw(cb, 3, 1, 0, 0);
                })
                .copy_image(
                    offscreen,
                    environment_map,
                    crate::ImageCopyDescBuilder::new(size, size)
                        .dst_base_array_layer(layer)
                        .dst_mip_level(mip)
                        .build(),
                )
                .build(device, graph);
        }
    }

    // Irradiance filter pass (mip 0 only)
    for layer in 0..6 {
        graph
            .add_pass_from_desc(
                format!("irradiance_filter_pass_layer_{layer}").as_str(),
                crate::PipelineDesc::builder()
                    .vertex_path("utopian/shaders/common/fullscreen.vert")
                    .fragment_path("utopian/shaders/ibl/irradiance_filter.frag"),
            )
            .read(environment_map)
            .write_layer(irradiance_map, layer)
            .uniforms("params", &(view_matrices[layer as usize], projection))
            .render(move |device, cb, _renderer, _pass, _resources| unsafe {
                let viewport = [render_utils::viewport(mip0_size, mip0_size)];
                device.handle.cmd_set_viewport(cb, 0, &viewport);
                device.handle.cmd_draw(cb, 3, 1, 0, 0);
            })
            .build(device, graph);
    }

    // Specular filter pass (all mip levels)
    for mip in 0..num_mips {
        let mip_size = (mip0_size as f32 * 0.5f32.powf(mip as f32)) as u32;

        for layer in 0..6 {
            graph
                .add_pass_from_desc(
                    format!("specular_filter_pass_layer_{layer}_mip_{mip}").as_str(),
                    crate::PipelineDesc::builder()
                        .vertex_path("utopian/shaders/ibl/fullscreen_with_pushconst.vert")
                        .fragment_path("utopian/shaders/ibl/specular_filter.frag"),
                )
                .read(environment_map)
                .write(offscreen)
                .uniforms("params", &(view_matrices[layer as usize], projection))
                .render(move |device, cb, _renderer, pass, resources| unsafe {
                    let viewport = [render_utils::viewport(mip_size, mip_size)];
                    device.handle.cmd_set_viewport(cb, 0, &viewport);

                    let roughness = mip as f32 / (num_mips - 1) as f32;

                    device.cmd_push_constants(
                        cb,
                        resources.pipeline(pass.pipeline_handle).pipeline_layout,
                        roughness,
                    );

                    device.handle.cmd_draw(cb, 3, 1, 0, 0);
                })
                .copy_image(
                    offscreen,
                    specular_map,
                    crate::ImageCopyDescBuilder::new(mip_size, mip_size)
                        .dst_base_array_layer(layer)
                        .dst_mip_level(mip)
                        .build(),
                )
                .build(device, graph);
        }
    }

    graph
        .add_pass_from_desc(
            "brdf_lut_pass",
            crate::PipelineDesc::builder()
                .vertex_path("utopian/shaders/common/fullscreen.vert")
                .fragment_path("utopian/shaders/ibl/brdf_lut.frag"),
        )
        .write(brdf_lut)
        .render(move |device, command_buffer, _, _, _| unsafe {
            device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
        })
        .build(device, graph);

    (environment_map, irradiance_map, specular_map, brdf_lut)
}
