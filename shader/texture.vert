#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (binding = 2) uniform UBO {
    mat4 projection_matrix;
} ubo;

layout (location = 0) in vec4 i_pos;
layout (location = 1) in vec2 i_uv;


layout (location = 0) out vec2 o_uv;
void main() {
    o_uv = i_uv;
    gl_Position = ubo.projection_matrix * i_pos;
}