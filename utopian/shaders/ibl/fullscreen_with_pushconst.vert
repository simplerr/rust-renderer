#version 450

// Todo: only supports push constants with vk::ShaderStageFlags::ALL currently
// and this is used by specular_filter.frag
layout(push_constant) uniform PushConsts {
   float roughness;
} pushConsts;

layout (location = 0) out vec2 outUV;

out gl_PerVertex
{
   vec4 gl_Position;
};

void main()
{
   float todo = pushConsts.roughness;
   outUV = vec2(gl_VertexIndex & 2, (gl_VertexIndex << 1) & 2);
   gl_Position = vec4(outUV * 2.0f - 1.0f, 0.0f, 1.0f);
}
