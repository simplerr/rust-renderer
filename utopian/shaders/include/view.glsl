
layout (std140, set = 1, binding = 0) uniform UBO_view
{
    mat4 view;
    mat4 projection;
    mat4 inverse_view;
    mat4 inverse_projection;
    vec3 eye_pos;
    uint samples_per_frame;
    vec3 sun_dir;
    uint total_samples;
    uint num_bounces;
    uint viewport_width;
    uint viewport_height;
    uint pad1;

    // render settings
    uint shadows_enabled;
    uint ssao_enabled;
    uint fxaa_enabled;
    uint cubemap_enabled;
} view;

// Due to gl_Position not being multiplied by -1 we need to flip the
// y axis of the uv coordinates. Todo: this should be possible to  get rid of.
#define ENABLE_UV_Y_FLIP
#ifdef ENABLE_UV_Y_FLIP
    #define FLIP_UV_Y(uv) vec2(uv.x, 1.0 - uv.y)
#else
    #define FLIP_UV_Y(uv) uv
#endif

float luminance(vec3 rgb)
{
   // Coefficents from the BT.709 standard
   return dot(rgb, vec3(0.2126f, 0.7152f, 0.0722f));
}

float linearToSrgb(float linearColor)
{
   if (linearColor < 0.0031308f) {
      return linearColor * 12.92f;
   }
   else {
      return 1.055f * float(pow(linearColor, 1.0f / 2.4f)) - 0.055f;
   }
}

vec3 linearToSrgb(vec3 linearColor)
{
   return vec3(linearToSrgb(linearColor.x), linearToSrgb(linearColor.y), linearToSrgb(linearColor.z));
}
