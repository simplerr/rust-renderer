#version 460
#extension GL_EXT_ray_tracing : enable
#extension GL_EXT_nonuniform_qualifier : enable

#include "include/bindless.glsl"
#include "include/random.glsl"

layout(location = 0) rayPayloadInEXT int rayPayload;

void main()
{
   rayPayload = 1;
}
