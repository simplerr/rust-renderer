#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// The engine unconditionally binds the bindless set at 0 and the view set at 1
// for every graphics pipeline, so the shader's pipeline layout must declare
// matching layouts at those indices. Including the standard headers is the
// canonical way to put them into shader reflection.
#include "include/bindless.glsl"
#include "include/view.glsl"

layout (location = 0) out vec4 _shadertoy_out_color;

layout (std140, set = 2, binding = 0) uniform UBO_shadertoy
{
    vec3 _iResolution;
    float _iTime;
    vec4 _iMouse;
    vec4 _iDate;
    float _iTimeDelta;
    int _iFrame;
    vec2 _pad;
} shadertoy_inputs;

#define iResolution shadertoy_inputs._iResolution
#define iTime       shadertoy_inputs._iTime
#define iMouse      shadertoy_inputs._iMouse
#define iDate       shadertoy_inputs._iDate
#define iTimeDelta  shadertoy_inputs._iTimeDelta
#define iFrame      shadertoy_inputs._iFrame

void mainImage(out vec4 fragColor, in vec2 fragCoord);

void main()
{
    // Shadertoy treats (0, 0) as the bottom-left of the image, but in Vulkan
    // gl_FragCoord has its origin at the top-left of the framebuffer. Flip Y
    // here so shaders that do `uv = fragCoord / iResolution.xy` render the
    // right way up without needing per-shader changes.
    vec2 fragCoord = vec2(gl_FragCoord.x, iResolution.y - gl_FragCoord.y);
    mainImage(_shadertoy_out_color, fragCoord);
}
