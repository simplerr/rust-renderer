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

float linearize_depth(float d, float zNear, float zFar)
{
    return zNear * zFar / (zFar + d * (zNear - zFar));
}

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

    /* Direct lighting */
    vec3 Lo = vec3(0.0);
    for (int i = 0; i < numLights; i++)
    {
       Lo += surfaceShading(pixel, lights[i], view.eye_pos.xyz, 1.0f);
    }

    // Todo: IBL
    vec3 ambient = vec3(0.03) * diffuse_color.rgb * occlusion;
    vec3 color = ambient + Lo;

    out_color = vec4(color, 1.0f);

    // Test shadow map...
    vec2 uv = vec2(gl_FragCoord.x / view.viewport_width, gl_FragCoord.y / view.viewport_height);
    float depth = texture(in_shadow_map, vec3(uv, 1)).r;
    depth = linearize_depth(depth, 0.01, 20000.0);
    out_color.rgb = vec3(depth);
}

