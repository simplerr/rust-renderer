use ash::vk;
use glam::{Mat4, Vec3};

use crate::image::ImageDesc;

pub fn setup_cubemap_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    base: &crate::VulkanBase,
    cubemap: crate::TextureId,
    enabled: bool,
) {
    let pipeline_handle = graph.create_pipeline(crate::PipelineDesc {
        vertex_path: "utopian/shaders/common/fullscreen.vert",
        fragment_path: "utopian/shaders/cubemap/cubemap.frag",
        vertex_input_binding_descriptions: vec![],
        vertex_input_attribute_descriptions: vec![],
        color_attachment_formats: vec![graph.resources.textures[cubemap].texture.image.format()],
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

    // Todo: get around the long lines to access image properties...

    let mip0_size = graph.resources.textures[cubemap].texture.image.width();
    let num_mips = graph.resources.textures[cubemap].texture.image.num_mips();

    let offscreen = graph.create_texture(
        "cubemap_offscreen",
        device,
        ImageDesc::new_2d(mip0_size, mip0_size, vk::Format::R32G32B32A32_SFLOAT).usage(
            vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::TRANSFER_SRC,
        ),
    );

    for mip in 0..num_mips {
        // let viewport_size = dimension * (0.5f32).powf(mip_level as f32);
        let mip_size = mip0_size as f32 * 0.5f32.powf(mip as f32);

        for layer in 0..6 {
            graph
                .add_pass(format!("cubemap_pass_layer_{layer}_mip_{mip}"), pipeline_handle)
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
                    cubemap,
                    vk::ImageCopy::builder()
                        .src_subresource(
                            vk::ImageSubresourceLayers::builder()
                                .mip_level(0)
                                .base_array_layer(0)
                                .layer_count(1)
                                .build(),
                        )
                        .src_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                        .dst_subresource(
                            vk::ImageSubresourceLayers::builder()
                                .mip_level(mip)
                                .base_array_layer(layer)
                                .layer_count(1)
                                .build(),
                        )
                        .dst_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
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
}
