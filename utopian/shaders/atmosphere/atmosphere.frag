#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"
#include "include/atmosphere.glsl"

layout (location = 0) in vec3 in_pos_l;

layout (location = 0) out vec4 out_color;

layout (set = 2, binding = 0) uniform samplerCube in_enviroment_map;

// Function to extract camera position from a view matrix
vec3 extractCameraPosition(mat4 viewMatrix) {
   mat4 inverseViewMatrix = inverse(viewMatrix);
   vec3 cameraPosition = vec3(inverseViewMatrix[3]);
   return cameraPosition;
}

void main()
{
   //vec3 rayStart = sharedVariables.eyePos.xyz;
   vec3 rayStart = extractCameraPosition(view.view);
   vec3 rayDir = normalize(in_pos_l);
   float rayLength = 999999999.0f;
   vec3 sunDir = view.sun_dir;
   vec3 lightColor = vec3(1.0f);

   vec3 color = vec3(0.0);

   if (view.cubemap_enabled == 1) {
      color = textureLod(in_enviroment_map, rayDir * vec3(1, -1, 1), 0).rgb;
   }
   else {
      vec3 transmittance;
      color = IntegrateScattering(rayStart, rayDir, rayLength, sunDir, lightColor, transmittance);
   }

   out_color = vec4(vec3(color), 1.0f);
}
