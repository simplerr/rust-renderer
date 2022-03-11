#version 460
#extension GL_EXT_ray_tracing : enable

#include "payload.glsl"

layout(location = 0) rayPayloadInEXT Payload rayPayload;

void main()
{
   const float t = 0.5*(normalize(gl_WorldRayDirectionEXT).y + 1.0);
   //const vec3 skyColor = mix(vec3(1.0), vec3(0.5, 0.7, 1.0), t);
   vec3 skyColor = vec3(1.0);

   rayPayload = Payload(vec4(skyColor, -1), vec4(0.0), 0);
}
