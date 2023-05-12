#version 460
#extension GL_EXT_ray_tracing : enable
#extension GL_EXT_nonuniform_qualifier : enable

#include "include/bindless.glsl"

layout(location = 0) rayPayloadInEXT bool rayPayload;

void main()
{
   rayPayload = true;
}
