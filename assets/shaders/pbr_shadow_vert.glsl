#version 450
#pragma shader_stage(vertex)

#include "rich.cginc"

layout(push_constant) uniform PushConsts {
    mat4 model;
} push_constants;


layout(location = 0) in vec3 in_pos;


void main() {
    vec4 loc_pos = push_constants.model * vec4(in_pos, 1.0);
    vec3 world_pos = loc_pos.xyz / loc_pos.w;
    gl_Position = ubo.light_matrix * vec4(world_pos, 1.0);
}

