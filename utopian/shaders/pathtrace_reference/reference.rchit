#version 460
#extension GL_EXT_ray_tracing : enable
#extension GL_EXT_nonuniform_qualifier : enable

#include "include/bindless.glsl"
#include "payload.glsl"
#include "random.glsl"

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
   vec3 normal = v0.normal.xyz * barycentrics.x + v1.normal.xyz * barycentrics.y + v2.normal.xyz * barycentrics.z;
   vec3 world_normal = normalize(vec3(normal.xyz * gl_WorldToObjectEXT));

   // Flip normal towards the incident ray direction
   if (dot(world_normal, gl_WorldRayDirectionEXT) > 0.0f) {
      world_normal = -world_normal;
   }

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
   else if (material.raytrace_properties.x == 2) {
      vec3 normalized_direction = normalize(gl_WorldRayDirectionEXT);
      const float dir_normal_dot = dot(normalized_direction, world_normal);
      const vec3 outward_normal = dir_normal_dot > 0 ? -world_normal : world_normal;
      float refraction_ratio = material.raytrace_properties.y;
      refraction_ratio = dir_normal_dot > 0 ? refraction_ratio : 1.0 / refraction_ratio;

      float cos_theta = min(dot(-1.0 * normalized_direction, outward_normal), 1.0);
      float sin_theta = sqrt(1.0 - cos_theta * cos_theta);

      bool cannot_refract = refraction_ratio * sin_theta > 1.0;
      float reflectance = schlick_reflectance(cos_theta, refraction_ratio);

      if (cannot_refract || reflectance > randomFloat(rayPayload.randomSeed)) {
         scatterDirection = reflect(normalized_direction, outward_normal);
      }
      else {
         scatterDirection = refract(normalized_direction, outward_normal, refraction_ratio);
      }

      isScattered = true;
      color = vec3(1.0);
   }
   // Diffuse light
   else {
      // Todo
      isScattered = false;
      color = vec3(1.0);
   }

   rayPayload = Payload(vec4(color, gl_HitTEXT), vec4(scatterDirection, isScattered ? 1 : 0), vec4(world_normal, 0.0), rayPayload.randomSeed);
}
