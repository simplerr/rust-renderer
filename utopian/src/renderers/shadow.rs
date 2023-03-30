use crate::camera;
use glam::{Mat4, Vec3, Vec4Swizzles};

pub fn setup_shadow_pass(
    device: &crate::Device,
    graph: &mut crate::Graph,
    shadow_map: crate::TextureId,
    sun_dir: glam::Vec3,
    camera: &camera::Camera,
    enabled: bool,
) -> ([glam::Mat4; 4], [f32; 4]) {
    puffin::profile_function!();

    let pipeline_handle = graph.create_pipeline(
        crate::PipelineDesc::builder()
            .vertex_path("utopian/shaders/shadow/shadow.vert")
            .fragment_path("utopian/shaders/shadow/shadow.frag")
            .default_primitive_vertex_bindings()
            .default_primitive_vertex_attributes()
            .build(),
    );

    // Todo:
    // Currently there is no good way to fully disable a pass so all this
    // code will allways run for now.

    const SHADOW_MAP_CASCADE_COUNT: u32 = 4;

    // Outputs
    let mut out_cascade_matrices = [glam::Mat4::IDENTITY; SHADOW_MAP_CASCADE_COUNT as usize];
    let mut out_split_depths = [0.0; SHADOW_MAP_CASCADE_COUNT as usize];

    let mut cascade_splits = [0.0; SHADOW_MAP_CASCADE_COUNT as usize];

    let near_clip = camera.get_near_plane();
    let far_clip = camera.get_far_plane();
    let clip_range = far_clip - near_clip;

    let min_z = near_clip;
    let max_z = near_clip + clip_range;

    let range = max_z - min_z;
    let ratio = max_z / min_z;

    let cascade_split_lambda = 0.927;

    // Calculate split depths based on view camera frustum
    // Based on method presented in https://developer.nvidia.com/gpugems/GPUGems3/gpugems3_ch10.html
    for i in 0..SHADOW_MAP_CASCADE_COUNT {
        let p = (i + 1) as f32 / SHADOW_MAP_CASCADE_COUNT as f32;
        let log = min_z * ratio.powf(p);
        let uniform = min_z + range * p;
        let d = cascade_split_lambda * (log - uniform) + uniform;
        cascade_splits[i as usize] = (d - near_clip) / clip_range;
    }

    // Calculate orthographic projection matrix for each cascade
    let mut last_split_dist = 0.0;
    for i in 0..SHADOW_MAP_CASCADE_COUNT {
        let split_dist = cascade_splits[i as usize];

        let mut frustum_corners: [Vec3; 8] = [
            Vec3::new(-1.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(-1.0, -1.0, 1.0),
        ];

        // Project frustum corners into world space
        let inv_cam = (camera.get_projection() * camera.get_view()).inverse();
        for i in 0..8 {
            let inv_corner = inv_cam * frustum_corners[i].extend(1.0);
            frustum_corners[i] = inv_corner.xyz() / inv_corner.w;
        }

        for i in 0..4 {
            let dist = frustum_corners[i + 4] - frustum_corners[i];
            frustum_corners[i + 4] = frustum_corners[i] + (dist * split_dist);
            frustum_corners[i] = frustum_corners[i] + (dist * last_split_dist);
        }

        // Get frustum center
        let frustum_center: Vec3 = frustum_corners.iter().sum::<Vec3>() / 8.0;

        let mut radius: f32 = 0.0;
        for i in 0..8 {
            let distance = (frustum_corners[i] - frustum_center).length();
            radius = radius.max(distance);
        }
        radius = f32::ceil(radius * 16.0) / 16.0;

        let max_extents = Vec3::new(radius, radius, radius);
        let min_extents = -max_extents;

        let light_view_matrix = Mat4::look_at_rh(
            frustum_center - sun_dir * min_extents.z,
            frustum_center,
            Vec3::Y,
        );

        let light_ortho_matrix = Mat4::orthographic_rh(
            min_extents.x,
            max_extents.x,
            min_extents.y,
            max_extents.y,
            -(max_extents.z - min_extents.z),
            max_extents.z - min_extents.z,
        );

        let view_projection_matrix = light_ortho_matrix * light_view_matrix;
        out_cascade_matrices[i as usize] = view_projection_matrix;
        out_split_depths[i as usize] = near_clip + split_dist * clip_range;

        last_split_dist = split_dist;

        graph
            .add_pass(format!("shadow_pass_{i}"), pipeline_handle)
            .uniforms("cascade_view_projection", &view_projection_matrix)
            .depth_attachment_layer(shadow_map, i as u32)
            .render(move |device, command_buffer, renderer, pass, resources| {
                // Todo: This is a hack to get around the fact that we can't properly disable a pass
                if enabled {
                    let pipeline = resources.pipeline(pass.pipeline_handle);

                    renderer.draw_meshes(device, command_buffer, pipeline.pipeline_layout);
                }
            })
            .build(&device, graph);
    }

    (out_cascade_matrices, out_split_depths)
}
