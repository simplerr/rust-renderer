#version 460
#extension GL_EXT_ray_tracing : enable

#include "include/atmosphere.glsl"
#include "include/view.glsl"
#include "payload.glsl"

layout(location = 0) rayPayloadInEXT Payload rayPayload;

void main()
{
   vec3 sky_color = vec3(1.0);

#ifndef FURNACE_TEST
   if (view.sky_enabled == 1) {
      vec3 light_dir = normalize(view.sun_dir);
      vec3 transmittance = vec3(0.0);
      sky_color = IntegrateScattering(gl_WorldRayOriginEXT, gl_WorldRayDirectionEXT, 999999999.0f, light_dir, vec3(1.0), transmittance);

      // Todo: we could use the atmosphere cubemap here

      // sky_color is in HDR range so clamp it for now to not get over exposure
      sky_color = min(sky_color, vec3(1.0));
   }
   else {
      sky_color = vec3(0.0);
   }
#endif

   rayPayload = Payload(vec4(sky_color, -1), vec4(0.0), vec4(0.0), 0);
}
