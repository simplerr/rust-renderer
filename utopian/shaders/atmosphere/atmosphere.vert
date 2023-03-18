#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"

layout (location = 0) in vec4 pos;
layout (location = 1) in vec4 normal;
layout (location = 2) in vec2 uv;
layout (location = 3) in vec4 color;
layout (location = 4) in vec4 tangent;

layout (set = 3, binding = 0) uniform UBO_constants
{
   mat4 projection;
   mat4 world;
} ubo_constants;

// layout(push_constant) uniform PushConsts {
//    mat4 view;
// } pushConsts;

layout (location = 0) out vec3 out_pos_l;

out gl_PerVertex
{
   vec4 gl_Position;
};

void main()
{
   out_pos_l = pos.xyz;

   // Removes the translation components of the matrix to always keep the skybox at the same distance
   //mat4 viewNoTranslation = mat4(mat3(pushConsts.view));
   mat4 viewNoTranslation = mat4(mat3(view.view));
   gl_Position = ubo_constants.projection * viewNoTranslation * ubo_constants.world * vec4(pos.xyz, 1.0);
}
