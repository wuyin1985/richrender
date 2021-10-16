#version 450
#pragma shader_stage(vertex)

#include "rich.cginc"

layout(push_constant) uniform PushConsts {
    mat4 model;
} push_constants;


layout(location = 0) in vec3 in_pos;

#ifdef IN_NORMAL
layout(location = 1) in vec3 in_normal;
#endif

#ifdef IN_TEX_COORD
layout(location = 2) in vec2 in_tex_coord;
#endif

#ifdef SKIN
layout(location = 3) in vec4 weights;
layout(location = 4) in uvec4 joints;

layout(binding = 0, set = 2) uniform SkinUBO {
    mat4 mat[512];
} skin;

#endif


#ifdef IN_TEX_COORD
layout(location = 0) out vec2 out_tex_coord;
#endif

layout(location = 1) out vec4 out_shadow_coord;

layout(location = 2) out vec3 out_camera_dir;

#ifdef IN_NORMAL
layout(location = 3) out vec3 out_normal;
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
    vec3 world_pos = loc_pos.xyz;
    out_camera_dir = ubo.camera_pos.xyz - world_pos;

    #ifdef IN_NORMAL
    out_normal = normalize(transpose(inverse(mat3(model))) * in_normal);
    #endif

    gl_Position = ubo.proj * ubo.view * loc_pos;

    #ifdef IN_TEX_COORD
    out_tex_coord = in_tex_coord;
    #endif

    out_shadow_coord = shadowBiasMat * ubo.light_matrix * loc_pos;
}

