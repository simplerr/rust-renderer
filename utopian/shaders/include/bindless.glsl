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

layout (set = 0, binding = 0) uniform sampler2D samplerColor[];

layout (std430, set = 0, binding = 1) readonly buffer VerticesSSBO
{
   Vertex vertices[];
} verticesSSBO[];

layout (scalar, set = 0, binding = 2) readonly buffer IndicesSSBO
{
   ivec3 indices[];
} indicesSSBO[];
