#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) in vec4 in_color;
layout (location = 0) out vec4 out_color;

layout (std140, set = 0, binding = 0) uniform UBO_test1
{
   vec4 color;
} test1;

layout (std140, set = 0, binding = 1) uniform UBO_test_frag2
{
   vec4 color;
} test2;

layout (std140, set = 5, binding = 100) uniform UBO_test_frag
{
   vec4 color;
} test_frag;

void main() {
    out_color = test_frag.color;
    out_color = in_color;
    out_color = test1.color;
}

