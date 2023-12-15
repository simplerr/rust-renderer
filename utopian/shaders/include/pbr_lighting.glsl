#extension GL_GOOGLE_include_directive : enable

// PBR code reused from https://github.com/simplerr/UtopianEngine

#include "include/brdf.glsl"



struct PixelParams
{
   vec3 position;
   vec3 baseColor;
   vec3 normal;
   vec3 F0;
   float metallic;
   float roughness;
   float occlusion;
};

vec3 surfaceShading(const PixelParams pixel, const Light light, const vec3 eyePos, float lightColorFactor)
{
   vec3 color = vec3(0.0f);

   /* Implementation from https://learnopengl.com/PBR/Theory */
   vec3 N = pixel.normal;
   vec3 V = normalize(eyePos - pixel.position);
   vec3 R = reflect(V, N);

   vec3 F0 = vec3(0.04);
   F0 = mix(F0, pixel.baseColor, pixel.metallic);

   vec3 L = vec3(0.0);
   float attenuation = 1.0f;
   vec3 posToLight = light.pos - pixel.position;

   if(light.type == 0.0f) // Directional light
   {
      L = normalize(light.dir * vec3(-1,1,-1));
      attenuation = 1.0f;
   }
   else if(light.type == 1.0f) // Point light
   {
      L = normalize(posToLight);
      float d = length(posToLight);
      attenuation = 1.0f / dot(light.att, vec3(1.0f, d, d*d));
   }
   else if(light.type == 2.0f) // Spot light
   {
      L = normalize(posToLight);
      float d = length(posToLight);
      float spot = pow(max(dot(L, normalize(light.dir)), 0.0f), light.spot);
      attenuation = spot / dot(light.att, vec3(1.0f, d, d*d));
   }

   // Reflectance equation
   vec3 Lo = vec3(0.0);

   vec3 H = normalize(V + L);
   vec3 radiance     = light.color.rgb * attenuation * lightColorFactor;

   // Cook-torrance brdf
   float NDF = DistributionGGX(N, H, pixel.roughness);
   float G   = GeometrySmith(N, V, L, pixel.roughness);
   vec3 F    = fresnelSchlick(max(dot(H, V), 0.0), F0);

   vec3 kS = F;
   vec3 kD = vec3(1.0) - kS;
   kD *= 1.0 - pixel.metallic;

   vec3 numerator    = NDF * G * F;
   float denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001;
   vec3 specular     = numerator / denominator;

   // Add to outgoing radiance Lo
   float NdotL = max(dot(N, L), 0.0);
   color = (kD * pixel.baseColor / PI + specular) * radiance * NdotL;

   return color;
}

vec3 imageBasedLighting(const PixelParams pixel, const vec3 eyePos, samplerCube in_irradiance_map,
                        samplerCube in_specular_map, sampler2D in_brdf_lut)
{
   vec3 V = normalize(eyePos - pixel.position);
   vec3 R = -reflect(V, pixel.normal); // Note: -1 indicates that the specular cubemp not being as expected

   vec3 F0 = vec3(0.04);
   F0 = mix(F0, pixel.baseColor, pixel.metallic);

   vec3 F = fresnelSchlickRoughness(max(dot(pixel.normal, V), 0.0), F0, pixel.roughness);
   vec3 kS = F;
   vec3 kD = 1.0 - kS;
   kD *= 1.0 - pixel.metallic;

   vec3 irradiance = texture(in_irradiance_map, pixel.normal).rgb;
   vec3 diffuse    = irradiance * pixel.baseColor;

   // Sample both the pre-filter map and the BRDF lut and combine them together as per the Split-Sum approximation to get the IBL specular part.
   // Note: 1 - roughness, same as Vulkan-glTF-PBR but differs from LearnOpenGL
   const float MAX_REFLECTION_LOD = 7.0;
   vec3 prefilteredColor = textureLod(in_specular_map, R, pixel.roughness * MAX_REFLECTION_LOD).rgb;
   vec2 brdf = texture(in_brdf_lut, vec2(max(dot(pixel.normal, V), 0.0), 1.0f - pixel.roughness)).rg;
   vec3 specular = prefilteredColor * (F * brdf.x + brdf.y);

   vec3 ambient = (kD * diffuse + specular) * pixel.occlusion;

   return ambient;
}
