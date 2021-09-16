#version 450

#include "rich.cginc"

layout(push_constant) uniform PushConsts {
    mat4 model;
} pushConsts;


layout(location = 0) in vec3 vPos;
layout(location = 1) in vec2 tex_coord;
layout(location = 2) in vec3 in_normal;

layout(location = 0) out vec2 fragTexCoord;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec3 out_camera_dir;

void main() {
    vec4 locPos = pushConsts.model * vec4(vPos, 1.0);
    locPos.y = -locPos.y;
    gl_Position = ubo.proj * ubo.view * locPos;
    fragTexCoord = tex_coord;

    out_camera_dir = ubo.camera_pos - vec3(locPos);
    out_normal = normalize(transpose(inverse(mat3(pushConsts.model))) * in_normal);
}

