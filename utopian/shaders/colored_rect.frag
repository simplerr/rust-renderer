#version 450
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"

layout (location = 0) in vec2 InTex;

layout (location = 0) out vec4 OutFragColor;

void main()
{
   OutFragColor = vec4(InTex, 0.0, 1.0);
}