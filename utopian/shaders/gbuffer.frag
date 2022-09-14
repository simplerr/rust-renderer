#version 450
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"

layout (location = 0) in vec2 InTex;

layout (location = 0) out vec4 out_gbuffer_position;
layout (location = 1) out vec4 out_gbuffer_normal;
layout (location = 2) out vec4 out_gbuffer_albedo;

void main()
{
   out_gbuffer_position = vec4(InTex, 0.0, 1.0);
   out_gbuffer_normal = vec4(1.0, 0.0, 0.0, 1.0);
   out_gbuffer_albedo = vec4(0.0, 1.0, 0.0, 1.0);
}