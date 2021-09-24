#version 450
#pragma shader_stage(fragment)
#include "rich.cginc"

layout(location = 0) out vec4 out_color;


void main() {
    out_color = vec4(1.0, 1.0, 1.0, 1.0);
}