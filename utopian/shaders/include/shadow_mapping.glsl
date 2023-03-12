#define SHADOW_MAP_CASCADE_COUNT 4

float linearize_depth(float d, float zNear, float zFar)
{
    return zNear * zFar / (zFar + d * (zNear - zFar));
}

float calculateShadow(vec3 position, out uint cascadeIndex)
{

   vec3 viewPosition = (view.view * vec4(position, 1.0f)).xyz;
   cascadeIndex = 0;
   for(uint i = 0; i < SHADOW_MAP_CASCADE_COUNT - 1; ++i) {
      if(viewPosition.z < -shadowmapParams.cascade_splits[i]) {
         cascadeIndex = i + 1;
      }
   }

   vec4 lightSpacePosition = shadowmapParams.view_projection_matrices[cascadeIndex] * vec4(position, 1.0f);
   vec4 projCoordinate = lightSpacePosition / lightSpacePosition.w;
   projCoordinate.xy = projCoordinate.xy * 0.5f + 0.5f;

   // Todo: Hack: Wtf: Why is this needed!?
   // Wasted hours on it........
   projCoordinate.y = 1.0 - projCoordinate.y;

   float shadow = 0.0f;
   vec2 texelSize = 1.0 / textureSize(in_shadow_map, 0).xy;
   int count = 0;
   int range = 1;
   for (int x = -range; x <= range; x++)
   {
      for (int y = -range; y <= range; y++)
      {
         // If fragment depth is outside frustum do no shadowing
         if (projCoordinate.z <= 1.0f && projCoordinate.z > -1.0f)
         {
            vec2 offset = vec2(x, y) * texelSize;
            float closestDepth = texture(in_shadow_map, vec3(projCoordinate.xy + offset, cascadeIndex)).r;
            float bias = 0.0005;
            const float shadowFactor = 0.3f;
            float testDepth = projCoordinate.z - bias;
            shadow += (testDepth > closestDepth ? shadowFactor : 1.0f);
         }
         else
         {
            shadow += 1.0f;
         }

         count++;
      }
   }

   shadow /= (count);

   return shadow;
}

vec3 cascade_index_to_debug_color(uint cascade_index) {
   switch(cascade_index) {
      case 0 :
         return vec3(1.0f, 0.25f, 0.25f);
      case 1 :
         return vec3(0.25f, 1.0f, 0.25f);
      case 2 :
         return vec3(0.25f, 0.25f, 1.0f);
      case 3 :
         return vec3(1.0f, 1.0f, 0.25f);
   }

   return vec3(1.0f, 1.0f, 1.0f);
}