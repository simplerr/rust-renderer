#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"
#include "include/atmosphere.glsl"

layout (location = 0) in vec2 in_uv;

layout (location = 0) out vec4 out_color;

layout (std140, set = 2, binding = 0) uniform UBO_parameters
{
   mat4 view;
   mat4 projection;
} params;

void main()
{
   vec3 worldDir = world_dir_from_uv(in_uv, params.view, params.projection);

   vec3 rayStart = extract_camera_position(view.view);
   vec3 rayDir = worldDir;
   float rayLength = 999999999.0f;
   vec3 sunDir = view.sun_dir;
   vec3 lightColor = vec3(1.0f);

   vec3 transmittance;
   vec3 color = IntegrateScattering(rayStart, rayDir, rayLength, sunDir, lightColor, transmittance);

   // For testing filtering:
   // color = sin((rayDir * 0.5 + 0.5) * 60.0);

   // color = vec3(in_uv, 0.0);
   // color = worldDir;

   out_color = vec4(vec3(color), 1.0f);
}
