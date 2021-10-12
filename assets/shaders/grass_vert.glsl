#version 450
#pragma shader_stage(vertex)

layout(location = 0) in vec4 v0_in;
layout(location = 1) in vec4 v1_in;
layout(location = 2) in vec4 v2_in;

layout(location = 0) out vec4 v0_tesc;
layout(location = 1) out vec4 v1_tesc;
layout(location = 2) out vec4 v2_tesc;



void main()
{
    v0_tesc = v0_in;
    v1_tesc = v1_in;
    v2_tesc = v2_in;

    gl_Position = vec4(v0_in.x, v0_in.y, v0_in.z, 1.0);
}
