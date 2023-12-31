#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (binding = 0) uniform UBO {
    vec3 color;
} ubo;

layout (binding = 1) uniform sampler2D sampler_color;

layout (location = 0) in vec2 o_uv;
layout (location = 0) out vec4 u_frag_color;

void main() {
    vec4 color = texture(sampler_color, o_uv);

    if (color.a < 1.0) {
        discard;
    }

    u_frag_color = color;
}
