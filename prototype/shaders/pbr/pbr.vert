#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

layout (location = 0) in vec3 pos;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec2 uv;
layout (location = 3) in vec4 color;
layout (location = 4) in vec4 tangent;

layout (location = 0) out vec3 out_pos;
layout (location = 1) out vec2 out_uv;
layout (location = 2) out vec3 out_normal;
layout (location = 3) out vec4 out_color;
layout (location = 4) out vec4 out_tangent;
layout (location = 5) out mat3 out_tbn;


layout (std140, set = 1, binding = 0) uniform UBO_camera
{
   mat4 view;
   mat4 projection;
   mat4 inverse_view;
   mat4 inverse_projection;
   vec3 eye_pos;
} camera;

layout(push_constant) uniform PushConsts {
   mat4 world;
   vec4 color;
   int diffuse_tex_id;
   int normal_tex_id;
   vec2 pad;
} pushConsts;


void main() {
    vec3 bitangentL = cross(normal, tangent.xyz);
    vec3 T = normalize(mat3(pushConsts.world) * tangent.xyz);
    vec3 B = normalize(mat3(pushConsts.world) * bitangentL);
    vec3 N = normalize(mat3(pushConsts.world) * normal);
    out_tbn = mat3(T, B, N);

    out_pos = (pushConsts.world * vec4(pos, 1.0)).xyz;
    out_uv = uv;
    out_color = color;
    out_normal = mat3(transpose(inverse(pushConsts.world))) * normal;
    out_tangent = tangent;
    out_color = pushConsts.color;
    gl_Position = camera.projection * camera.view * pushConsts.world * vec4(pos, 1.0);
}
