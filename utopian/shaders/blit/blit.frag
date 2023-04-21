#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"
#include "include/pbr_lighting.glsl"

layout (location = 0) in vec2 in_uv;

layout (location = 0) out vec4 out_color;

layout (set = 2, binding = 0) uniform sampler2D in_texture;

void main()
{
    vec2 uv = FLIP_UV_Y(in_uv);
    out_color = vec4(texture(in_texture, uv).rgb, 1.0);

    if (uv.x > 0.95 && uv.y > 0.95) {
        out_color = vec4(1, 1, 0, 1);
    }
}

