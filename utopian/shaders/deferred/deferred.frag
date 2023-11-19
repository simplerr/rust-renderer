#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"
#include "include/pbr_lighting.glsl"

layout (location = 0) in vec2 in_uv;

layout (location = 0) out vec4 out_color;

layout (set = 3, binding = 0) uniform texture2D in_gbuffer_position;
layout (set = 3, binding = 1) uniform texture2D in_gbuffer_normal;
layout (set = 3, binding = 2) uniform texture2D in_gbuffer_albedo;
layout (set = 3, binding = 3) uniform texture2D in_gbuffer_pbr;
layout (set = 3, binding = 4) uniform texture2DArray in_shadow_map;
layout (set = 3, binding = 5) uniform texture2D in_rt_shadows;
layout (set = 3, binding = 6) uniform texture2D in_rt_reflections;
layout (set = 3, binding = 7) uniform texture2D in_ssao;
layout (set = 3, binding = 8) uniform textureCube in_irradiance_map;
layout (set = 3, binding = 9) uniform textureCube in_specular_map;
layout (set = 3, binding = 10) uniform texture2D in_brdf_lut;

// Todo: set=2 should be dedicated to input textures but the shader reflection
// does not support gaps in the descriptor sets
layout (std140, set = 4, binding = 0) uniform UBO_shadowmapParams
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

vec4 lightColor = vec4(vec3(50.0f), 1.0f);
vec4 red = vec4(50.0, 0.0, 0.0, 1.0);
vec4 green = vec4(0.0, 50.0, 0.0, 1.0);
const int numLights = 3;

Light lights[numLights] = {
    Light(lightColor, vec3(1.0f, 1.0f, 3.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,1), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)),
    Light(red, vec3(8.0f, 6.0f, 0.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,1), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)),
    Light(green, vec3(8.0f, 1.0f, 0.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,3), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)),
   /* Light(lightColor, vec3(-2.0f, 1.0f, -2.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,1), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)), */
   /* Light(lightColor, vec3(-2.0f, 2.0f, -2.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,1), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)) */
};

void main() {
    vec2 uv = FLIP_UV_Y(in_uv);

    uint material_index = uint(texture(sampler2D(in_gbuffer_pbr, defaultSampler), uv).a);
    Material material = materialsSSBO.materials[material_index];

    vec3 position = texture(sampler2D(in_gbuffer_position, defaultSampler), uv).rgb;
    vec3 normal = texture(sampler2D(in_gbuffer_normal, defaultSampler), uv).rgb;
    vec3 diffuse_color = texture(sampler2D(in_gbuffer_albedo, defaultSampler), uv).rgb;
    float metallic = texture(sampler2D(in_gbuffer_pbr, defaultSampler), uv).r;
    float roughness = texture(sampler2D(in_gbuffer_pbr, defaultSampler), uv).g;
    float occlusion = texture(sampler2D(in_gbuffer_pbr, defaultSampler), uv).b;
    float ssao = texture(sampler2D(in_ssao, defaultSampler), in_uv).r;

    roughness *= material.roughness_factor;
    metallic *= material.metallic_factor;

    // From sRGB space to Linear space
    diffuse_color.rgb = pow(diffuse_color.rgb, vec3(2.2));

    PixelParams pixel;
    pixel.position = position;
    pixel.baseColor = diffuse_color.rgb * material.base_color_factor.rgb;
    pixel.normal = normal;
    pixel.metallic = metallic;
    pixel.roughness = roughness;
    pixel.occlusion = occlusion;

    /* Direct lighting */
    vec3 Lo = vec3(0.0);

    Light sun_light = Light(vec4(1.0f), vec3(0.0f), 0.0f, view_ubo.sun_dir * vec3(-1, 1, -1), 0.0f, vec3(1.0), 0.0f, vec3(0.0f), 0.0f, vec4(0.0f));
    Lo += surfaceShading(pixel, sun_light, view_ubo.eye_pos.xyz, 1.0f);

    // for (int i = 0; i < numLights; i++)
    // {
    //    Lo += surfaceShading(pixel, lights[i], view_ubo.eye_pos.xyz, 1.0f);
    // }

    vec3 ambient = vec3(0.03) * diffuse_color.rgb * occlusion;

    if (view_ubo.ibl_enabled == 1)
    {
        ambient = imageBasedLighting(pixel, view_ubo.eye_pos.xyz, in_irradiance_map, in_specular_map, in_brdf_lut);
    }

    vec3 color = ambient + Lo;

    if (view_ubo.raytracing_supported == 1 && material.raytrace_properties.x == 1) {
        vec3 reflectedColor = texture(sampler2D(in_rt_reflections, defaultSampler), uv).rgb;
        color = mix(color, reflectedColor, 1.0);
    }

    // Shadow
    if (view_ubo.shadows_enabled == 1) {
        uint cascadeIndex = 0;
        float shadow = calculateShadow(position, cascadeIndex);
        color = color * shadow;

        //#define CASCADE_DEBUG
        #ifdef CASCADE_DEBUG
            color.rgb *= cascade_index_to_debug_color(cascadeIndex);
        #endif
    }
    else if (view_ubo.raytracing_supported == 1) {
        float shadow = texture(sampler2D(in_rt_shadows, defaultSampler), uv).r;
        color = color * max(shadow, 0.3);
    }

    if (view_ubo.ssao_enabled == 1) {
        color *= ssao;
    }

    out_color = vec4(color, 1.0f);
}

