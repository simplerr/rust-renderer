#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"
#include "include/brdf.glsl"

layout (location = 0) in vec2 in_uv;

layout (location = 0) out vec4 out_color;

layout (set = 3, binding = 0) uniform textureCube in_enviroment_map;

// Todo: could be push constants instead
layout (std140, set = 4, binding = 0) uniform UBO_parameters
{
   mat4 view;
   mat4 projection;
} params;

layout(push_constant) uniform PushConsts {
   float roughness;
} pushConsts;

#define PI 3.1415926535897932384626433832795

// Normal Distribution function
float D_GGX(float dotNH, float roughness)
{
   float alpha = roughness * roughness;
   float alpha2 = alpha * alpha;
   float denom = dotNH * dotNH * (alpha2 - 1.0) + 1.0;
   return (alpha2)/(PI * denom*denom);
}

// Generates an prefiltered specular cube from an environment map using convolution
// Source: https://github.com/SaschaWillems/Vulkan/blob/master/data/shaders/glsl/pbrtexture/prefilterenvmap.frag
vec3 prefilterEnvMap(vec3 R, float roughness)
{
   vec3 N = R;
   vec3 V = R;
   vec3 color = vec3(0.0);
   float totalWeight = 0.0;
   float envMapDim = float(textureSize(samplerCube(in_enviroment_map, defaultSampler), 0).s);
   const int numSamples = 32;
   for(uint i = 0u; i < numSamples; i++) {
      vec2 Xi = hammersley2d(i, numSamples);
      vec3 H = importanceSample_GGX(Xi, roughness, N);
      vec3 L = 2.0 * dot(V, H) * H - V;
      float dotNL = clamp(dot(N, L), 0.0, 1.0);
      if(dotNL > 0.0) {
         // Filtering based on https://placeholderart.wordpress.com/2015/07/28/implementation-notes-runtime-environment-map-filtering-for-image-based-lighting/

         float dotNH = clamp(dot(N, H), 0.0, 1.0);
         float dotVH = clamp(dot(V, H), 0.0, 1.0);

         // Probability Distribution Function
         float pdf = D_GGX(dotNH, roughness) * dotNH / (4.0 * dotVH) + 0.0001;
         // Slid angle of current smple
         float omegaS = 1.0 / (float(numSamples) * pdf);
         // Solid angle of 1 pixel across all cube faces
         float omegaP = 4.0 * PI / (6.0 * envMapDim * envMapDim);
         // Biased (+1.0) mip level for better result
         float mipLevel = roughness == 0.0 ? 0.0 : max(0.5 * log2(omegaS / omegaP) + 1.0, 0.0f);
         color += textureLod(samplerCube(in_enviroment_map, defaultSampler), L, mipLevel).rgb * dotNL;
         totalWeight += dotNL;

      }
   }

   return (color / totalWeight);
}

void main()
{
   vec3 normal = world_dir_from_uv(in_uv, params.view, params.projection);
   out_color = vec4(prefilterEnvMap(normal, pushConsts.roughness), 1.0);
}