#extension GL_EXT_scalar_block_layout : enable
#extension GL_EXT_nonuniform_qualifier : enable

struct Vertex
{
   vec4 pos;
   vec4 normal;
   vec2 uv;
   vec4 color;
   vec4 tangent;
};

struct Material
{
   uint diffuse_map;
   uint normal_map;
   uint metallic_roughness_map;
   uint occlusion_map;
   vec4 base_color_factor;
   float metallic_factor;
   float roughness_factor;
   vec2 padding;

   // Ray tracing properties
   // x = type (0 = lambertian, 1 = metal, 2 = dielectric, 3 = diffuse light)
   // y = metal -> fuzz, dielectric -> index of refractions
   vec4 raytrace_properties;
};

struct Mesh
{
   uint vertex_buffer;
   uint index_buffer;
   uint material;
};

layout (set = 0, binding = 0) uniform texture2D samplerColor[];

layout (std430, set = 0, binding = 1) readonly buffer VerticesSSBO
{
   Vertex vertices[];
} verticesSSBO[];

layout (scalar, set = 0, binding = 2) readonly buffer IndicesSSBO
{
   ivec3 indices[];
} indicesSSBO[];

layout (scalar, set = 0, binding = 3) readonly buffer MaterialsSSBO
{
   Material materials[];
} materialsSSBO;

layout (scalar, set = 0, binding = 4) readonly buffer MeshesSSBO
{
   Mesh meshes[];
} meshesSSBO;

layout (set = 2, binding = 0) uniform sampler defaultSampler;