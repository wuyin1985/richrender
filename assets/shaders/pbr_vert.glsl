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


#ifdef IN_TEX_COORD
layout(location = 0) out vec2 out_tex_coord;
#endif

layout(location = 1) out vec4 out_shadow_coord;

layout(location = 2) out vec3 out_camera_dir;

#ifdef IN_NORMAL
layout(location = 3) out vec3 out_normal;
#endif


void main() {
    vec4 loc_pos = push_constants.model * vec4(in_pos, 1.0);
    vec3 world_pos = loc_pos.xyz / loc_pos.w;
    out_camera_dir = ubo.camera_pos - world_pos;
    #ifdef IN_NORMAL
    out_normal = normalize(transpose(inverse(mat3(push_constants.model))) * in_normal);
    #endif

    gl_Position = ubo.proj * ubo.view * vec4(world_pos, 1.0);

    #ifdef IN_TEX_COORD
    out_tex_coord = in_tex_coord;
    #endif

    out_shadow_coord = ubo.light_matrix * vec4(world_pos, 1.0);
    out_shadow_coord.y = -out_shadow_coord.y;
    out_shadow_coord.xy = out_shadow_coord.xy * 0.5 + 0.5;
}

