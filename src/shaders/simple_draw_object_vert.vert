#version 450

#include "testlib.cginc"

layout(push_constant) uniform PushConsts {
    mat4 model;
} pushConsts;


layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 view;
    mat4 proj;
} ubo;

layout(location = 0) in vec3 vPos;
layout(location = 1) in vec2 tex_coord;

layout(location = 0) out vec2 fragTexCoord;

void main() {
    vec4 locPos = pushConsts.model * vec4(vPos, 1.0);
    locPos.y = -locPos.y;
    gl_Position = ubo.proj * ubo.view * locPos;
    fragTexCoord = tex_coord;
}

