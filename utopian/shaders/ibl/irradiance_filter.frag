#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"

layout (location = 0) in vec2 in_uv;

layout (location = 0) out vec4 out_color;

layout (set = 2, binding = 0) uniform samplerCube in_enviroment_map;

layout (std140, set = 3, binding = 0) uniform UBO_parameters
{
   mat4 view;
   mat4 projection;
} params;

#define PI 3.1415926535897932384626433832795

// Generates an irradiance cube from an environment map using convolution
// Source: https://learnopengl.com/PBR/IBL/Diffuse-irradiance
void main()
{
   vec3 irradiance = vec3(0.0);

   vec3 normal = world_dir_from_uv(in_uv, params.view, params.projection);
   vec3 up    = vec3(0.0, 1.0, 0.0);
   vec3 right = normalize(cross(up, normal));
   up         = normalize(cross(normal, right));

   float sampleDelta = 0.025;
   float nrSamples = 0.0;
   for(float phi = 0.0; phi < 2.0 * PI; phi += sampleDelta)
   {
      for(float theta = 0.0; theta < 0.5 * PI; theta += sampleDelta)
      {
         // Spherical to cartesian (in tangent space)
         vec3 tangentSample = vec3(sin(theta) * cos(phi),  sin(theta) * sin(phi), cos(theta));
         // Tangent space to world
         vec3 sampleVec = tangentSample.x * right + tangentSample.y * up + tangentSample.z * normal;

         irradiance += texture(in_enviroment_map, sampleVec).rgb * cos(theta) * sin(theta);
         nrSamples++;
      }
   }
   irradiance = PI * irradiance * (1.0 / float(nrSamples));

   out_color = vec4(irradiance, 1.0f);
}
