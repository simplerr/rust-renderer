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
   color *= material.base_color_factor.rgb;

   vec3 scatterDirection;
   bool isScattered = false;

   // Lambertian
   if (material.raytrace_properties.x == 0) {
      scatterDirection = world_normal + randomPointInUnitSphere(rayPayload.randomSeed);
      isScattered = dot(gl_WorldRayDirectionEXT, world_normal) < 0;
   }
   // Metal
   else if (material.raytrace_properties.x == 1) {
      scatterDirection = reflect(normalize(gl_WorldRayDirectionEXT), world_normal);
      scatterDirection += material.raytrace_properties.y * randomPointInUnitSphere(rayPayload.randomSeed);

      // Note: the dot product below should be used but it's causing weird artifacts at sphere edges
      isScattered = true; // dot(scatterDirection, world_normal) > 0;
      color = vec3(1.0); // Note: Hardcode white color
   }
   // Dielectric
   else {
      color = vec3(1,0,0);
   }

   rayPayload = Payload(vec4(color, gl_HitTEXT), vec4(scatterDirection, isScattered ? 1 : 0), rayPayload.randomSeed);
}
