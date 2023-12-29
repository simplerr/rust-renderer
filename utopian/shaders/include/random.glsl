
// From https://nvpro-samples.github.io/vk_mini_path_tracer
// and https://github.com/boksajak/referencePT

uint jenkinsHash(uint x) {
   x += x << 10;
   x ^= x >> 6;
   x += x << 3;
   x ^= x >> 11;
   x += x << 15;
   return x;
}

uint initRNG(uvec2 pixelCoords, uvec2 resolution, uint frameNumber)
{
   uint seed = uint(dot(pixelCoords, uvec2(1, resolution.x))) ^ jenkinsHash(frameNumber);
   return jenkinsHash(seed);
}

// Random number generation using pcg32i_random_t, using inc = 1. Our random state is a uint.
uint stepRNG(uint rngState)
{
   return rngState * 747796405 + 1;
}

// Steps the RNG and returns a floating-point value between 0 and 1 inclusive.
float randomFloat(inout uint rngState)
{
   // Condensed version of pcg_output_rxs_m_xs_32_32, with simple conversion to floating-point [0,1].
   rngState  = stepRNG(rngState);
   uint word = ((rngState >> ((rngState >> 28) + 4)) ^ rngState) * 277803737;
   word      = (word >> 22) ^ word;
   return float(word) / 4294967295.0f;
}

vec3 randomPointInUnitSphere(inout uint rngState)
{
   for (;;)
   {
      vec3 point = 2 * vec3(randomFloat(rngState), randomFloat(rngState), randomFloat(rngState)) - 1;
      if (dot(point, point) < 1)
      {
         return point;
      }
   }
}

vec2 randomPointInUnitDisk(inout uint rngState)
{
   for (;;)
   {
      vec2 point = 2 * vec2(randomFloat(rngState), randomFloat(rngState)) - 1;
      if (dot(point, point) < 1)
      {
         return point;
      }
   }
}

// Todo: use this in the future

// inline vec3 random_cosine_direction() {
//    auto r1 = random_double();
//    auto r2 = random_double();

//    auto phi = 2*pi*r1;
//    auto x = cos(phi)*sqrt(r2);
//    auto y = sin(phi)*sqrt(r2);
//    auto z = sqrt(1-r2);

//    return vec3(x, y, z);
// }

// // Samples a direction within a hemisphere oriented along +Z axis with a cosine-weighted distribution 
// // Source: "Sampling Transformations Zoo" in Ray Tracing Gems by Shirley et al.
// // A derivation and explanation is available at https://raytracing.github.io/books/RayTracingTheRestOfYourLife.html#generatingrandomdirections
// vec3 sampleHemisphere(vec2 u, out float pdf)
// {
//    float a = sqrt(u.x);
//    float b = TWO_PI * u.y;

//    vec3 result = vec3(
//       a * cos(b),
//       a * sin(b),
//       sqrt(1.0f - u.x));

//    pdf = result.z * ONE_OVER_PI;

//    return result;
// }
