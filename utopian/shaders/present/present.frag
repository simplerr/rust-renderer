#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"
#include "include/pbr_lighting.glsl"

layout (location = 0) in vec2 in_uv;

layout (location = 0) out vec4 out_color;

layout (set = 2, binding = 0) uniform sampler2D in_forward_texture;
layout (set = 2, binding = 1) uniform sampler2D in_deferred_texture;
layout (set = 2, binding = 2) uniform sampler2DArray in_shadow_map;

layout(std140, set = 3, binding = 0) uniform FXAA_settings
{
   vec4 enabled_debug_threshold;
} settings_fxaa;

#include "include/fxaa.glsl"

void main() {
    vec2 uv = vec2(in_uv.x, 1.0 - in_uv.y);

    vec3 color;
    if (true || uv.x < 0.5) {
        //color = texture(in_forward_texture, uv).rgb;
        color = fxaa(uv, in_forward_texture);
    }
    else {
        //color = texture(in_deferred_texture, uv).rgb;
        color = fxaa(uv, in_deferred_texture);
    }

    /* Tonemapping */
    color = color / (color + vec3(1.0));
    color = pow(color, vec3(1.0/2.2));

    if (uv.x > 0.75 && uv.y < 0.25) {
        color = texture(in_shadow_map, vec3(in_uv * 4.0, 0.0)).rgb;
    }
    else if (uv.x > 0.75 && uv.y < 0.50) {
        color = texture(in_shadow_map, vec3(in_uv * 4.0, 1.0)).rgb;
    }
    else if (uv.x > 0.75 && uv.y < 0.75) {
        color = texture(in_shadow_map, vec3(in_uv * 4.0, 2.0)).rgb;
    }
    else if (uv.x > 0.75 && uv.y < 1.0) {
        color = texture(in_shadow_map, vec3(in_uv * 4.0, 3.0)).rgb;
    }

    out_color = vec4(color, 1.0);
}

