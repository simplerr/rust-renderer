#version 450
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"

layout (location = 0) in vec3 in_pos;
layout (location = 1) in vec2 in_uv;
layout (location = 2) in vec3 in_normal;
layout (location = 3) in vec4 in_color;
layout (location = 4) in vec4 in_tangent;
layout (location = 5) in mat3 in_tbn;

layout (location = 0) out vec4 out_gbuffer_position;
layout (location = 1) out vec4 out_gbuffer_normal;
layout (location = 2) out vec4 out_gbuffer_albedo;
layout (location = 3) out vec4 out_gbuffer_pbr;

layout(push_constant) uniform PushConsts {
    mat4 world;
    vec4 color;
    uint mesh_index;
    ivec3 pad;
} pushConsts;

void main()
{
    Mesh mesh = meshesSSBO.meshes[pushConsts.mesh_index];
    Material material = materialsSSBO.materials[mesh.material];

    vec4 diffuse_color = texture(samplerColor[material.diffuse_map], in_uv);
    vec4 normal_map = texture(samplerColor[material.normal_map], in_uv);
    float metallic = texture(samplerColor[material.metallic_roughness_map], in_uv).b;
    float roughness = texture(samplerColor[material.metallic_roughness_map], in_uv).g;
    float occlusion = texture(samplerColor[material.occlusion_map], in_uv).r;

    // From sRGB space to Linear space
    diffuse_color.rgb = pow(diffuse_color.rgb, vec3(2.2));

    vec3 normal = normalize(in_normal);
    if (in_tangent.xyz != vec3(0.0f))
    {
        normal = normalize(normal_map.xyz * 2.0 - 1.0);
        normal = normalize(in_tbn * normal);
    }

    out_gbuffer_position = vec4(in_pos, 1.0);
    out_gbuffer_normal = vec4(normal, 1.0);
    out_gbuffer_albedo = vec4(diffuse_color.rgb * material.base_color_factor.rgb, 1.0);
    out_gbuffer_pbr = vec4(metallic, roughness, occlusion, mesh.material);
}
