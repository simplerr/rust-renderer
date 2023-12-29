#version 460
#extension GL_EXT_ray_tracing : enable

#include "include/atmosphere.glsl"
#include "include/view.glsl"

layout(location = 0) rayPayloadInEXT int rayPayload;

void main()
{
   rayPayload = 0;
}
