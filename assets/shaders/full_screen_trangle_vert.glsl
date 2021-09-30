#version 450
#pragma shader_stage(vertex)

layout (location = 0) out vec2 outUV;

void main() {
    gl_Position = ubo.light_matrix * push_constants.model * vec4(in_pos, 1.0);
}

