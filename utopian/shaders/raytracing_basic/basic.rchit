#version 460
#extension GL_EXT_ray_tracing : enable
#extension GL_EXT_nonuniform_qualifier : enable

#include "include/bindless.glsl"

layout(location = 0) rayPayloadInEXT vec3 hitValue;
hitAttributeEXT vec2 attribs;

void main()
{
   const vec3 barycentrics = vec3(1.0f - attribs.x - attribs.y, attribs.x, attribs.y);
   hitValue = barycentrics;

   ivec3 indices = indicesSSBO[0].indices[gl_PrimitiveID];
   Vertex v0 = verticesSSBO[0].vertices[indices.x];
   Vertex v1 = verticesSSBO[0].vertices[indices.y];
   Vertex v2 = verticesSSBO[0].vertices[indices.z];

   vec3 normal = v0.normal.xyz * barycentrics.x + v1.normal.xyz * barycentrics.y + v2.normal.xyz * barycentrics.z;
   vec3 world_normal = normalize(vec3(normal.xyz * gl_WorldToObjectEXT));

   hitValue = vec3(world_normal);
}
