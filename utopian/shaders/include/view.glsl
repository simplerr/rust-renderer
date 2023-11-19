
layout (std140, set = 1, binding = 0) uniform UBO_view
{
    mat4 view_mat;
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
    float time;

    // render settings
    uint shadows_enabled;
    uint ssao_enabled;
    int fxaa_enabled;
    uint cubemap_enabled;
    uint ibl_enabled;
    uint marching_cubes_enabled;
    uint rebuild_tlas;
    uint raytracing_supported;
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

vec3 extract_camera_position(mat4 viewMatrix) {
   mat4 inverseViewMatrix = inverse(viewMatrix);
   vec3 cameraPosition = vec3(inverseViewMatrix[3]);
   return cameraPosition;
}

vec3 world_dir_from_ndc(vec3 ndc, mat4 view, mat4 projection)
{
   vec4 clipSpace = vec4(ndc, 1.0);
   vec4 viewSpace = inverse(projection) * clipSpace;
   viewSpace.w = 0.0;
   vec4 worldSpace = inverse(view) * viewSpace;
   vec3 worldDir = normalize(worldSpace.xyz);

   return worldDir;
}

vec3 world_dir_from_uv(vec2 uv, mat4 view, mat4 projection)
{
   return world_dir_from_ndc(vec3(uv, 0.0) * 2.0 - 1.0, view, projection);
}

// Clever offset_ray function from Ray Tracing Gems chapter 6
// Offsets the ray origin from current position p, along normal n (which must be geometric normal)
// so that no self-intersection can occur.
vec3 offsetRay(const vec3 p, const vec3 n)
{
   const float origin = 1.0f / 32.0f;
   const float float_scale = 1.0f / 65536.0f;
   const float int_scale = 256.0f;

   ivec3 of_i = ivec3(int_scale * n.x, int_scale * n.y, int_scale * n.z);

   vec3 p_i = vec3(
      intBitsToFloat(floatBitsToInt(p.x) + ((p.x < 0) ? -of_i.x : of_i.x)),
      intBitsToFloat(floatBitsToInt(p.y) + ((p.y < 0) ? -of_i.y : of_i.y)),
      intBitsToFloat(floatBitsToInt(p.z) + ((p.z < 0) ? -of_i.z : of_i.z)));

   return vec3(abs(p.x) < origin ? p.x + float_scale * n.x : p_i.x,
      abs(p.y) < origin ? p.y + float_scale * n.y : p_i.y,
      abs(p.z) < origin ? p.z + float_scale * n.z : p_i.z);
}
