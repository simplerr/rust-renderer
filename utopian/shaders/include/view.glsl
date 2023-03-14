
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

// Due to gl_Position not being multiplied by -1 we need to flip the
// y axis of the uv coordinates. Todo: this should be possible to  get rid of.
#define ENABLE_UV_Y_FLIP
#ifdef ENABLE_UV_Y_FLIP
    #define FLIP_UV_Y(uv) vec2(uv.x, 1.0 - uv.y)
#else
    #define FLIP_UV_Y(uv) uv
#endif
