#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

layout (location = 0) in vec3 pos;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec2 uv;
layout (location = 3) in vec4 color;
layout (location = 4) in vec4 tangent;

layout (location = 0) out vec2 out_uv;
layout (location = 1) out vec3 out_normal;
layout (location = 2) out vec4 out_color;
layout (location = 3) out vec4 out_tangent;

layout (std140, set = 1, binding = 0) uniform UBO_camera
{
   mat4 view;
   mat4 projection;
   vec4 eye_pos;
} camera;

layout(push_constant) uniform PushConsts {
   mat4 world;
   vec4 color;
   int diffuse_tex_id;
   int normal_tex_id;
   vec2 pad;
} pushConsts;


void main() {
    out_uv = uv;
    out_color = color;
    out_normal = normal;
    out_tangent = tangent;
    //out_color = camera.eyePos;
    //out_color = test1.color;
    out_color = pushConsts.color;
    gl_Position = camera.projection * camera.view * pushConsts.world * vec4(pos, 1.0);
}
