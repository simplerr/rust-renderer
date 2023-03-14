#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"

layout (location = 0) in vec2 in_uv;

layout (location = 0) out vec4 out_color;

layout (set = 2, binding = 0) uniform sampler2D in_gbuffer_position;
layout (set = 2, binding = 1) uniform sampler2D in_gbuffer_normal;

const int KERNEL_SIZE = 32;

// layout (std140, set = 0, binding = 0) uniform UBO_parameters
// {
//    vec4 kernelSamples[KERNEL_SIZE];
// } ubo;

layout(std140, set = 3, binding = 0) uniform UBO_settings
{
   float radius;
   float bias;
} settings_ubo;

// Hardcoded kernel samples for the moment, will be replaced by a random kernel
// These are generated from UtopianEngine
vec4 kernelSamples[KERNEL_SIZE] = vec4[](
   vec4(-0.68217, 0.23565, 0.48243, 0.0),
   vec4(-0.14448, 0.01628, 0.22807, 0.0),
   vec4(0.00604, 0.01909, 0.0127, 0.0),
   vec4(0.09733, 0.39072, 0.7324, 0.0),
   vec4(0.06055, 0.87847, 0.33303, 0.0),
   vec4(0.00734, 0.19034, 0.13091, 0.0),
   vec4(-0.01377, 0.01745, 0.00399, 0.0),
   vec4(0.01468, 0.16627, 0.09108, 0.0),
   vec4(-0.10093, -0.08015, 0.06625, 0.0),
   vec4(-0.27125, -0.39937, 0.0601, 0.0),
   vec4(-0.06181, -0.03065, 0.01213, 0.0),
   vec4(-0.40189, -0.48095, 0.21808, 0.0),
   vec4(0.04027, -0.05818, 0.26542, 0.0),
   vec4(-0.33535, -0.07516, 0.24997, 0.0),
   vec4(0.32748, -0.18112, 0.27292, 0.0),
   vec4(0.53962, -0.03361, 0.58926, 0.0),
   vec4(-0.09598, -0.25424, 0.35754, 0.0),
   vec4(-0.17368, 0.01261, 0.23964, 0.0),
   vec4(0.1283, 0.12573, 0.16467, 0.0),
   vec4(-0.34418, 0.19403, 0.70285, 0.0),
   vec4(-0.09686, -0.0928, 0.11447, 0.0),
   vec4(0.32727, -0.49713, 0.17518, 0.0),
   vec4(0.12345, 0.13862, 0.23822, 0.0),
   vec4(-0.39258, -0.31128, 0.67374, 0.0),
   vec4(0.03308, 0.07616, 0.03422, 0.0),
   vec4(-0.31777, 0.1885, 0.40808, 0.0),
   vec4(-0.17464, 0.28096, 0.11686, 0.0),
   vec4(-0.50199, -0.49002, 0.2709, 0.0),
   vec4(0.38629, 0.15627, 0.56716, 0.00),
   vec4(0.06649, -0.05762, 0.0857, 0.00),
   vec4(-0.1065, -0.11726, 0.10818, 0.00),
   vec4(0.53236, -0.5286, 0.45444, 0.00)
);

void main()
{
   // Note: not sure why this does not need to be flipped with FLIP_UV_Y
   vec2 uv = in_uv;

   // Get G-Buffer values
   vec3 positionWorld = texture(in_gbuffer_position, uv).xyz;
   vec3 fragPosView = (view.view * vec4(positionWorld, 1.0f)).xyz;

   // The position texture is cleared with 1 so this is a way to detect if we are in the skybox
   if (positionWorld == vec3(1.0)) {
      out_color.r = 1.0;
      return;
   }

   mat4 normalMatrix = transpose(inverse(view.view));
   vec3 normalWorld = texture(in_gbuffer_normal, uv).rgb;
   vec3 normalView = normalize((normalMatrix * vec4(normalWorld, 0.0)).xyz);

   // Todo:
   // Get a random vector using a noise lookup
   vec3 randomVec = vec3(1, 1, 0);
   
   // Create TBN matrix
   vec3 tangent = normalize(randomVec - normalView * dot(randomVec, normalView));
   vec3 bitangent = cross(tangent, normalView);
   mat3 TBN = mat3(tangent, bitangent, normalView);

   // Calculate occlusion value
   float occlusion = 0.0f;
   for(int i = 0; i < KERNEL_SIZE; i++)
   {
      vec3 samplePos = TBN * kernelSamples[i].xyz;
      samplePos = fragPosView + samplePos * settings_ubo.radius;
      
      // Project to NDC
      vec4 offset = vec4(samplePos, 1.0f);
      offset = view.projection * offset;
      offset.xyz /= offset.w;
      offset.xyz = offset.xyz * 0.5f + 0.5f;
      offset.xy = FLIP_UV_Y(offset.xy);

      float sampleDepth = (view.view * vec4(texture(in_gbuffer_position, offset.xy).xyz, 1.0f)).z;

      float rangeCheck = smoothstep(0.0f, 1.0f, settings_ubo.radius / abs(fragPosView.z - sampleDepth));
      occlusion += (sampleDepth >= samplePos.z ? 1.0f : 0.0f) * rangeCheck;
   }

   float strength = 1.6;
   occlusion = 1.0 - (occlusion / float(KERNEL_SIZE)) * strength;

   out_color.r = occlusion;
}
