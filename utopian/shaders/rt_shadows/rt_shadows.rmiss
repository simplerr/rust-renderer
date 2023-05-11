#version 460
#extension GL_EXT_ray_tracing : enable

#include "include/atmosphere.glsl"
#include "include/view.glsl"
#include "payload.glsl"

layout(location = 0) rayPayloadInEXT Payload rayPayload;

void main()
{
   rayPayload = Payload(vec4(-1), vec4(0.0), vec4(0.0), 0);
}
