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

void main() {
    vec2 uv = vec2(in_uv.x, in_uv.y - 1.0);

    if (uv.x < 0.5) {
        out_color = texture(in_forward_texture, uv);
    }
    else {
        out_color = texture(in_deferred_texture, uv);
    }
}

