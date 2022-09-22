#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"
#include "include/pbr_lighting.glsl"

layout (location = 0) in vec2 in_uv;

layout (location = 0) out vec4 out_color;

layout (set = 2, binding = 0) uniform sampler2D in_gbuffer_position;
layout (set = 2, binding = 1) uniform sampler2D in_gbuffer_normal;
layout (set = 2, binding = 2) uniform sampler2D in_gbuffer_albedo;
layout (set = 2, binding = 3) uniform sampler2D in_gbuffer_pbr;

layout (std140, set = 3, binding = 0) uniform UBO_parameters
{
    vec3 color;
} test_params;

layout(push_constant) uniform PushConsts {
    mat4 world;
    vec4 color;
    uint mesh_index;
    ivec3 pad;
} pushConsts;

vec4 lightColor = vec4(vec3(50.0f), 1.0f);
vec4 red = vec4(50.0, 0.0, 0.0, 1.0);
vec4 green = vec4(0.0, 50.0, 0.0, 1.0);
const int numLights = 4;

Light lights[numLights] = {
    Light(vec4(3.0f), vec3(0.0f, 1.0f, -2.0f), 0.0f, vec3(1.0f), 0.0f, vec3(0.2,0.001399,0.0), 0.0f, vec3(0.0f), 0.0f, vec4(0.0f)),
    Light(lightColor, vec3(1.0f, 1.0f, 3.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,1), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)),
    Light(red, vec3(8.0f, 6.0f, 0.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,1), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)),
    Light(green, vec3(8.0f, 1.0f, 0.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,3), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)),
   /* Light(lightColor, vec3(-2.0f, 1.0f, -2.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,1), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)), */
   /* Light(lightColor, vec3(-2.0f, 2.0f, -2.0f), 0.0f, vec3(0.0f), 0.0f, vec3(0,0,1), 1.0f, vec3(0.0f), 0.0f, vec4(0.0f)) */
};

void main() {
    Mesh mesh = meshesSSBO.meshes[pushConsts.mesh_index];
    Material material = materialsSSBO.materials[mesh.material];

    vec2 uv = vec2(in_uv.x, in_uv.y - 1.0);
    vec3 position = texture(in_gbuffer_position, uv).rgb;
    vec3 normal = texture(in_gbuffer_normal, uv).rgb;
    vec3 diffuse_color = texture(in_gbuffer_albedo, uv).rgb;
    float metallic = texture(in_gbuffer_pbr, uv).r;
    float roughness = texture(in_gbuffer_pbr, uv).g;
    float occlusion = texture(in_gbuffer_pbr, uv).b;

    PixelParams pixel;
    pixel.position = position;
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

