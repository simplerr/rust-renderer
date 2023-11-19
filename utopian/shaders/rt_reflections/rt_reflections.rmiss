#version 460
#extension GL_EXT_ray_tracing : enable

#include "include/atmosphere.glsl"
#include "include/view.glsl"

layout(location = 0) rayPayloadInEXT vec3 rayPayload;

void main()
{
#ifdef FURNACE_TEST
   vec3 sky_color = vec3(1.0);
#else
   vec3 light_dir = normalize(view_ubo.sun_dir);
   vec3 transmittance = vec3(0.0);
   vec3 sky_color = IntegrateScattering(gl_WorldRayOriginEXT, gl_WorldRayDirectionEXT, 999999999.0f, light_dir, vec3(1.0), transmittance);

   // Todo: we could use the atmosphere cubemap here

   // sky_color is in HDR range so clamp it for now to not get over exposure
   sky_color = min(sky_color, vec3(1.0));
#endif

   rayPayload = sky_color;
}
