#version 450
#pragma shader_stage(vertex)

#include "rich.cginc"

layout(push_constant) uniform PushConsts {
    mat4 model;
} push_constants;


layout(location = 0) in vec3 in_pos;

#ifdef SKIN
layout(location = 3) in vec4 weights;
layout(location = 4) in uvec4 joints;

layout(binding = 0, set = 1) uniform SkinUBO {
    mat4 mat[512];
} skin;

#endif


void main() {
    mat4 model = push_constants.model;

    #ifdef SKIN
    model *= weights.x * skin.mat[joints.x]
    + weights.y * skin.mat[joints.y]
    + weights.z * skin.mat[joints.z]
    + weights.w * skin.mat[joints.w];
    #endif

    vec4 loc_pos = model * vec4(in_pos, 1.0);
    gl_Position = ubo.light_matrix * loc_pos;
}

