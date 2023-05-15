#version 460
#extension GL_EXT_ray_tracing : enable
#extension GL_EXT_nonuniform_qualifier : enable

#include "include/bindless.glsl"
#include "payload.glsl"

layout(location = 0) rayPayloadInEXT Payload rayPayload;
hitAttributeEXT vec2 attribs;

float schlick_reflectance(float cosine, float ref_idx)
{
   // Schlick's approximation
   float r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
   r0 = r0 * r0;
   return r0 + (1.0 - r0) * pow(1.0 - cosine, 5.0);
}

void main()
{
   Mesh mesh = meshesSSBO.meshes[gl_InstanceCustomIndexEXT];
   Material material = materialsSSBO.materials[mesh.material];

   ivec3 indices = indicesSSBO[mesh.index_buffer].indices[gl_PrimitiveID];
   Vertex v0 = verticesSSBO[mesh.vertex_buffer].vertices[indices.x];
   Vertex v1 = verticesSSBO[mesh.vertex_buffer].vertices[indices.y];
   Vertex v2 = verticesSSBO[mesh.vertex_buffer].vertices[indices.z];

   const vec3 barycentrics = vec3(1.0f - attribs.x - attribs.y, attribs.x, attribs.y);
   vec2 uv = v0.uv * barycentrics.x + v1.uv * barycentrics.y + v2.uv * barycentrics.z;

   vec3 color = texture(samplerColor[material.diffuse_map], uv).xyz;
   color *= material.base_color_factor.rgb;

   rayPayload = Payload(vec4(color, gl_HitTEXT), vec4(0.0), vec4(0.0), 0);
}
