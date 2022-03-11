#version 460
#extension GL_EXT_ray_tracing : enable
#extension GL_EXT_nonuniform_qualifier : enable

#include "include/bindless.glsl"
#include "payload.glsl"
#include "random.glsl"

layout(location = 0) rayPayloadInEXT Payload rayPayload;
hitAttributeEXT vec2 attribs;

void main()
{
   Mesh mesh = meshesSSBO.meshes[gl_InstanceCustomIndexEXT];
   Material material = materialsSSBO.materials[mesh.material];

   ivec3 indices = indicesSSBO[mesh.index_buffer].indices[gl_PrimitiveID];
   Vertex v0 = verticesSSBO[mesh.vertex_buffer].vertices[indices.x];
   Vertex v1 = verticesSSBO[mesh.vertex_buffer].vertices[indices.y];
   Vertex v2 = verticesSSBO[mesh.vertex_buffer].vertices[indices.z];

   const vec3 barycentrics = vec3(1.0f - attribs.x - attribs.y, attribs.x, attribs.y);
   vec3 normal = v0.normal.xyz * barycentrics.x + v1.normal.xyz * barycentrics.y + v2.normal.xyz * barycentrics.z;
   vec3 world_normal = normalize(vec3(normal.xyz * gl_WorldToObjectEXT));
   vec2 uv = v0.uv * barycentrics.x + v1.uv * barycentrics.y + v2.uv * barycentrics.z;
   vec3 color = texture(samplerColor[material.diffuse_map], uv).xyz;

   vec3 scatterDirection = world_normal + randomPointInUnitSphere(rayPayload.randomSeed);
   const bool isScattered = dot(gl_WorldRayDirectionEXT, world_normal) < 0;

   rayPayload = Payload(vec4(color, gl_HitTEXT), vec4(scatterDirection, isScattered ? 1 : 0), rayPayload.randomSeed);
}
