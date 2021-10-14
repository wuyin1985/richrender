#version 450
#pragma shader_stage(tesseval)
#include "rich.cginc"

layout(quads, equal_spacing, cw) in;

// TODO: Declare tessellation evaluation shader inputs and outputs
layout(location = 0) in vec4 v0_tese[];
layout(location = 1) in vec4 v1_tese[];
layout(location = 2) in vec4 v2_tese[];

layout(location = 0) out vec4 f_normal;
layout(location = 1) out vec3 f_pos_world;
layout(location = 2) out vec4 out_shadow_coord;

float rand(vec2 co){
    return fract(sin(dot(co, vec2(12.9898, 78.233))) * 43758.5453);
}

void main()
{
    float u = gl_TessCoord.x;
    float v = gl_TessCoord.y;

    // Use u and v to parameterize along the grass blade and output positions for each vertex of the grass blade
    vec3 a = v0_tese[0].xyz + v * (v1_tese[0].xyz - v0_tese[0].xyz);
    vec3 b = v1_tese[0].xyz + v * (v2_tese[0].xyz - v1_tese[0].xyz);
    vec3 center = a + v * (b - a);

    float width = v2_tese[0].w;
    float angle = v0_tese[0].w;

    vec3 bitangent = vec3(cos(angle), 0.0, sin(angle));
    float scaling = 1.2-v;
    vec3 c0 = center - width * scaling * bitangent;
    vec3 c1 = center + width * scaling * bitangent;

    vec3 tangent = normalize(b - a);
    f_normal.xyz = normalize(bitangent);
    f_normal.w = v;//for fragment shading

    //float t = u - u * v * v; // quadratic shape
    //float t = u + 0.5*v -u*v; // triangular shape
    float tao = 0.75;
    float t = 0.5 + (u-0.5)*(1.0 - (max(v-tao, 0.0) / (1.0-tao)));// triangle tip shape

    f_pos_world = mix(c0, c1, t);

    gl_Position = ubo.proj * ubo.view * vec4(f_pos_world, 1.0);
    out_shadow_coord = shadowBiasMat * ubo.light_matrix * vec4(f_pos_world, 1.0);
}
