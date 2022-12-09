
layout (std140, set = 1, binding = 0) uniform UBO_view
{
    mat4 view;
    mat4 projection;
    mat4 inverse_view;
    mat4 inverse_projection;
    vec3 eye_pos;
    uint samples_per_frame;
    uint total_samples;
    uint num_bounces;
    uint viewport_width;
    uint viewport_height;
    vec3 sun_dir;
} view;
