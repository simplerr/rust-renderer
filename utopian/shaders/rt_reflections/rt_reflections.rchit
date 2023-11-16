#version 460
#extension GL_EXT_ray_tracing : enable
#extension GL_EXT_nonuniform_qualifier : enable

#include "include/bindless.glsl"
#include "include/view.glsl"
#include "include/pbr_lighting.glsl"

layout (set = 3, binding = 4) uniform textureCube in_irradiance_map;
layout (set = 3, binding = 5) uniform textureCube in_specular_map;
layout (set = 3, binding = 6) uniform texture2D in_brdf_lut;

layout(location = 0) rayPayloadInEXT vec3 rayPayload;
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
   vec3 position = v0.pos.xyz * barycentrics.x + v1.pos.xyz * barycentrics.y + v2.pos.xyz * barycentrics.z;
   vec3 normal = v0.normal.xyz * barycentrics.x + v1.normal.xyz * barycentrics.y + v2.normal.xyz * barycentrics.z;
   vec3 world_position = vec3(position.xyz * gl_ObjectToWorldEXT );
   vec3 world_normal = normalize(vec3(normal.xyz * gl_WorldToObjectEXT));

   // Flip normal towards the incident ray direction
   if (dot(world_normal, gl_WorldRayDirectionEXT) > 0.0f) {
      world_normal = -world_normal;
   }

   vec2 uv = v0.uv * barycentrics.x + v1.uv * barycentrics.y + v2.uv * barycentrics.z;

   vec3 color = texture(sampler2D(samplerColor[material.diffuse_map], defaultSampler), uv).xyz;
   color *= material.base_color_factor.rgb;

   if (view.ibl_enabled == 1)
   {
      PixelParams pixel;
      pixel.position = world_position;
      pixel.baseColor = color;
      pixel.normal = world_normal;
      pixel.metallic = texture(sampler2D(samplerColor[material.metallic_roughness_map], defaultSampler), uv).b;
      pixel.roughness = texture(sampler2D(samplerColor[material.metallic_roughness_map], defaultSampler), uv).g;
      pixel.occlusion = texture(sampler2D(samplerColor[material.occlusion_map], defaultSampler), uv).r;

      rayPayload = imageBasedLighting(pixel, view.eye_pos.xyz, in_irradiance_map, in_specular_map, in_brdf_lut);
   }
   else {
      rayPayload = vec3(0.1) * color;
   }
}
