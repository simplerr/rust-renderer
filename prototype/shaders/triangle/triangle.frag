#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (location = 0) in vec2 in_uv;
layout (location = 1) in vec3 in_normal;
layout (location = 2) in vec4 in_color;

layout (location = 0) out vec4 out_color;

layout (set = 0, binding = 0) uniform sampler2D samplerColor[];

layout(push_constant) uniform PushConsts {
   mat4 world;
   vec4 color;
   int diffuse_tex_id;
   int normal_tex_id;
   vec2 pad;
} pushConsts;

void main() {
    out_color = in_color;

    out_color = texture(samplerColor[pushConsts.diffuse_tex_id], in_uv);
    //out_color = texture(samplerColor[pushConsts.normal_tex_id], in_uv);
    //out_color = vec4(in_normal, 1.0);
}

