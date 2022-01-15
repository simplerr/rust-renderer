#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

layout (location = 0) in vec3 pos;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec2 uv;
layout (location = 3) in vec4 color;

layout (location = 0) out vec2 out_uv;
layout (location = 1) out vec3 out_normal;
layout (location = 2) out vec4 out_color;

layout (std140, set = 0, binding = 0) uniform UBO_test1
{
   vec4 color;
} test1;

layout (std140, set = 0, binding = 1) uniform UBO_camera
{
   mat4 view;
   mat4 projection;
   vec4 eye_pos;
} camera;

layout (std140, set = 1, binding = 1) uniform UBO_mouse
{
   vec2 mouseUV;
   float time;
} mouse;

layout (set = 2, binding = 0) uniform sampler2D diffuseSampler;
layout (set = 2, binding = 1) uniform sampler2D normalSampler;
layout (set = 2, binding = 2) uniform sampler2D specularSampler;

layout (set = 4, binding = 0, r32f) uniform writeonly image3D sdfImageOutput;

layout (set = 3, binding = 0) uniform isampler2D edgeTableTex;
layout (set = 3, binding = 1) uniform isampler2D triangleTableTex;
layout (set = 3, binding = 2) uniform sampler3D sdfImage;

layout(push_constant) uniform PushConsts {
   //mat4 world;
   vec4 color;
} pushConsts;


void main() {
    out_uv = uv;
    out_color = color;
    out_normal = normal;
    //out_color = camera.eyePos;
    //out_color = test1.color;
    //out_color = pushConsts.color;
    gl_Position = camera.projection * camera.view * vec4(pos, 1.0);
}
