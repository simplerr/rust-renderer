#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"
#include "include/pbr_lighting.glsl"

layout (location = 0) in vec2 in_uv;

layout (location = 0) out vec4 out_color;

layout (set = 3, binding = 0) uniform texture2D in_color_texture;

layout(std140, set = 4, binding = 0) uniform FXAA_settings
{
   vec4 enabled_debug_threshold;
} settings_fxaa;

#include "include/fxaa.glsl"

void main() {
    vec2 uv = FLIP_UV_Y(in_uv);

    vec3 color = vec3(0.0);

    if (view_ubo.fxaa_enabled == 1) {
        color = fxaa(in_color_texture, uv);
    }
    else {
        color = texture(sampler2D(in_color_texture, defaultSampler), uv).rgb;
    }

    /* Tonemapping */
    // color = color / (color + vec3(1.0));
    color = linearToSrgb(color);

    out_color = vec4(color, 1.0);
}

