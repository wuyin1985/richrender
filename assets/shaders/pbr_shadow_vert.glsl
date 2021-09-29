#version 450
#pragma shader_stage(vertex)

#include "rich.cginc"

layout(push_constant) uniform PushConsts {
    mat4 model;
} push_constants;


layout(location = 0) in vec3 in_pos;


void main() {
    gl_Position = ubo.light_matrix * push_constants.model * vec4(in_pos, 1.0);
}

