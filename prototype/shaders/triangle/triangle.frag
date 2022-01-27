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
   int diffuse_map;
   int normal_map;
   int metallic_roughness_map;
   int occlusion_map;
} pushConsts;

void main() {
    out_color = in_color;

    float metallic = texture(samplerColor[pushConsts.metallic_roughness_map], in_uv).b;
    float roughness = texture(samplerColor[pushConsts.metallic_roughness_map], in_uv).g;
    float occlusion = texture(samplerColor[pushConsts.occlusion_map], in_uv).r;

    out_color = vec4(vec3(metallic), 1.0);
    out_color = texture(samplerColor[pushConsts.diffuse_map], in_uv);
}

