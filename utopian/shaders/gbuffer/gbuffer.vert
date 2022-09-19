#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_GOOGLE_include_directive : enable

#include "include/bindless.glsl"
#include "include/view.glsl"

layout (location = 0) in vec4 pos;
layout (location = 1) in vec4 normal;
layout (location = 2) in vec2 uv;
layout (location = 3) in vec4 color;
layout (location = 4) in vec4 tangent;

layout (location = 0) out vec3 out_pos;
layout (location = 1) out vec2 out_uv;
layout (location = 2) out vec3 out_normal;
layout (location = 3) out vec4 out_color;
layout (location = 4) out vec4 out_tangent;
layout (location = 5) out mat3 out_tbn;

layout(push_constant) uniform PushConsts {
   mat4 world;
   vec4 color;
   uint mesh_index;
   ivec3 pad;
} pushConsts;

void main() {
    Mesh mesh = meshesSSBO.meshes[pushConsts.mesh_index];
    Vertex vertex = verticesSSBO[mesh.vertex_buffer].vertices[gl_VertexIndex];

#define BINDLESS
#ifdef BINDLESS
    vec3 bitangentL = cross(vertex.normal.xyz, vertex.tangent.xyz);
    vec3 T = normalize(mat3(pushConsts.world) * vertex.tangent.xyz);
    vec3 B = normalize(mat3(pushConsts.world) * bitangentL);
    vec3 N = normalize(mat3(pushConsts.world) * vertex.normal.xyz);
    out_tbn = mat3(T, B, N);

    out_pos = (pushConsts.world * vec4(vertex.pos.xyz, 1.0)).xyz;
    out_uv = vertex.uv;
    out_color = vertex.color;
    out_normal = mat3(transpose(inverse(pushConsts.world))) * vertex.normal.xyz;
    out_tangent = vertex.tangent;
    gl_Position = camera.projection * camera.view * pushConsts.world * vec4(vertex.pos.xyz, 1.0);
#else
    vec3 bitangentL = cross(normal.xyz, tangent.xyz);
    vec3 T = normalize(mat3(pushConsts.world) * tangent.xyz);
    vec3 B = normalize(mat3(pushConsts.world) * bitangentL);
    vec3 N = normalize(mat3(pushConsts.world) * normal.xyz);
    out_tbn = mat3(T, B, N);

    out_pos = (pushConsts.world * vec4(pos.xyz, 1.0)).xyz;
    out_uv = uv;
    out_color = color;
    out_normal = mat3(transpose(inverse(pushConsts.world))) * normal.xyz;
    out_tangent = tangent;
    gl_Position = camera.projection * camera.view * pushConsts.world * vec4(pos.xyz, 1.0);
#endif

}
