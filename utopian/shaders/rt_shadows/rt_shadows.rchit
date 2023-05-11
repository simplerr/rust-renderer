#version 460
#extension GL_EXT_ray_tracing : enable
#extension GL_EXT_nonuniform_qualifier : enable

#include "include/bindless.glsl"
#include "payload.glsl"

layout(location = 0) rayPayloadInEXT Payload rayPayload;
hitAttributeEXT vec2 attribs;

void main()
{
   rayPayload = Payload(vec4(1.0), vec4(1.0), vec4(1.0), 0);
}
