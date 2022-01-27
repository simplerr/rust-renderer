#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable
#extension GL_EXT_nonuniform_qualifier : enable

#include "pbr_lighting.glsl"

layout (location = 0) in vec3 in_pos;
layout (location = 1) in vec2 in_uv;
layout (location = 2) in vec3 in_normal;
layout (location = 3) in vec4 in_color;
layout (location = 4) in vec4 in_tangent;
layout (location = 5) in mat3 in_tbn;

layout (location = 0) out vec4 out_color;

layout (set = 0, binding = 0) uniform sampler2D samplerColor[];

layout (std140, set = 1, binding = 0) uniform UBO_camera
{
    mat4 view;
    mat4 projection;
    vec3 eye_pos;
} camera;

layout(push_constant) uniform PushConsts {
    mat4 world;
    vec4 color;
    int diffuse_map;
    int normal_map;
    int metallic_roughness_map;
    int occlusion_map;
} pushConsts;

vec4 lightColor = vec4(vec3(50.0f), 1.0f);
const int numLights = 2;

Light lights[numLights] = {
    Light(vec4(3.0f), vec3(0.0f, 1.0f, -2.0f), 0.0f, vec3(1.0f), 0.0f, vec3(0.2,0.001399,0.0), 0.0f, vec3(0.0f), 0.0f, vec4(0.0f)),
    Light(lightColor, vec3(1.0f, 1.0f, 1.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,1), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)),
   /* Light(lightColor, vec3(0.0f, 2.0f, -2.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,1), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)), */
   /* Light(lightColor, vec3(-2.0f, 1.0f, -2.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,1), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)), */
   /* Light(lightColor, vec3(-2.0f, 2.0f, -2.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,1), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)) */
};

void main() {
    vec4 diffuse_color = texture(samplerColor[pushConsts.diffuse_map], in_uv);
    vec4 normal_map = texture(samplerColor[pushConsts.normal_map], in_uv);
    float metallic = texture(samplerColor[pushConsts.metallic_roughness_map], in_uv).b;
    float roughness = texture(samplerColor[pushConsts.metallic_roughness_map], in_uv).g;
    float occlusion = texture(samplerColor[pushConsts.occlusion_map], in_uv).r;

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
    pixel.baseColor = diffuse_color.rgb;
    pixel.normal = normal;
    pixel.metallic = metallic;
    pixel.roughness = roughness;

    /* Direct lighting */
    vec3 Lo = vec3(0.0);
    for (int i = 0; i < numLights; i++)
    {
       Lo += surfaceShading(pixel, lights[i], camera.eye_pos.xyz, 1.0f);
    }

    // Todo: IBL
    vec3 ambient = vec3(0.03) * diffuse_color.rgb * occlusion;

    vec3 color = ambient + Lo;

    /* Tonemapping */
    color = color / (color + vec3(1.0));
    color = pow(color, vec3(1.0/2.2));

    out_color = vec4(color, 1.0f);
}

