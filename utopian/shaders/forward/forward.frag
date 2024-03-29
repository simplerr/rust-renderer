#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"
#include "include/pbr_lighting.glsl"

layout (location = 0) in vec3 in_pos;
layout (location = 1) in vec2 in_uv;
layout (location = 2) in vec3 in_normal;
layout (location = 3) in vec4 in_color;
layout (location = 4) in vec4 in_tangent;
layout (location = 5) in mat3 in_tbn;

layout (location = 0) out vec4 out_color;

layout (set = 2, binding = 0) uniform sampler2DArray in_shadow_map;

// Todo: set=2 should be dedicated to input textures but the shader reflection
// does not support gaps in the descriptor sets
layout (std140, set = 3, binding = 0) uniform UBO_shadowmapParams
{
    mat4 view_projection_matrices[4];
    vec4 cascade_splits;
} shadowmapParams;

#include "include/shadow_mapping.glsl"

layout(push_constant) uniform PushConsts {
    mat4 world;
    vec4 color;
    uint mesh_index;
    ivec3 pad;
} pushConsts;

void main() {
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

    PixelParams pixel;
    pixel.position = in_pos;
    pixel.baseColor = diffuse_color.rgb * material.base_color_factor.rgb;
    pixel.normal = normal;
    pixel.metallic = metallic;
    pixel.roughness = roughness;
    pixel.occlusion = occlusion;

    /* Direct lighting */
    vec3 Lo = vec3(0.0);

    Light sun_light = Light(vec4(1.0f), vec3(0.0f), 0.0f, view.sun_dir * vec3(-1, 1, -1), 0.0f, vec3(1.0), 0.0f, vec3(0.0f), 0.0f, vec4(0.0f));
    Lo += surfaceShading(pixel, sun_light, view.eye_pos.xyz, 1.0f);

    for (int i = 0; i < view.num_lights; i++)
    {
       Lo += surfaceShading(pixel, lightsSSBO.lights[i], view.eye_pos.xyz, 1.0f);
    }

    // Todo: IBL
    vec3 ambient = vec3(0.03) * diffuse_color.rgb * occlusion;
    vec3 color = ambient + Lo;

    // Shadow
    if (view.shadows_enabled == 1) {
        uint cascadeIndex = 0;
        float shadow = calculateShadow(in_pos, cascadeIndex);
        color = color * shadow;

        //#define CASCADE_DEBUG
        #ifdef CASCADE_DEBUG
            color.rgb *= cascade_index_to_debug_color(cascadeIndex);
        #endif
    }


    out_color = vec4(color, 1.0f);
}

