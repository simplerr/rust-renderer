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

// Function to extract camera position from a view matrix
vec3 extractCameraPosition(mat4 viewMatrix) {
   mat4 inverseViewMatrix = inverse(viewMatrix);
   vec3 cameraPosition = vec3(inverseViewMatrix[3]);
   return cameraPosition;
}

void main()
{
   vec3 ndc = vec3(in_uv, 0.0) * 2.0 - 1.0;
   vec4 clipSpace = vec4(ndc, 1.0);
   vec4 viewSpace = inverse(params.projection) * clipSpace;
   viewSpace.w = 0.0;
   vec4 worldSpace = inverse(params.view) * viewSpace;
   vec3 worldDir = normalize(worldSpace.xyz);

   vec3 rayStart = extractCameraPosition(view.view);
   vec3 rayDir = worldDir;
   float rayLength = 999999999.0f;
   vec3 sunDir = view.sun_dir;
   vec3 lightColor = vec3(1.0f);

   vec3 transmittance;
   vec3 color = IntegrateScattering(rayStart, rayDir, rayLength, sunDir, lightColor, transmittance);

   // color = vec3(in_uv, 0.0);
   // color = worldDir;

   out_color = vec4(vec3(color), 1.0f);
}
