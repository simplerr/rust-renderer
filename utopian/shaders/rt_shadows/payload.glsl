
struct Payload
{
   vec4 colorDistance; // rgb + t
   vec4 scatterDirection; // xyz + is scattered
   vec4 normal;
   uint randomSeed;
};
