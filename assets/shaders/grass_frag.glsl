#version 450
#pragma shader_stage(fragment)
#include "rich.cginc"


layout(location = 0) in vec4 f_normal;
layout(location = 1) in vec3 f_pos_world;
layout(location = 2) in vec4 in_shadow_coord;

layout(location = 0) out vec4 outColor;

layout (set = 1, binding = 0) uniform sampler2D shadow_map;

float textureProj(vec4 shadowCoord, vec2 off)
{
    float shadow = 0.0;
    float dist = texture(shadow_map, shadowCoord.xy + off).r;
    if (shadowCoord.w > 0 && dist < shadowCoord.z)
    {
        shadow = 1.0;
    }
    return shadow;
}

void main()
{
    vec3 fragColor = vec3(0.0, 1.0, 0.0);

    vec3 darkGreen = vec3(0.0823, 0.3176, 0.0);
    vec3 lightGreen = vec3(0.1647, 0.5882, 0.0);
    vec3 color = mix(darkGreen, lightGreen, clamp(f_normal.w, 0.0, 1.0));

    float lambert = clamp(abs(dot(normalize(f_normal.xyz), normalize(ubo.light_dir.xyz))), 0.0, 1.0);
    float ambient = 0.2f;

    float shadow = textureProj(in_shadow_coord / in_shadow_coord.w, vec2(0.0, 0.0));

    fragColor = color * ambient + (1 - shadow) * color;

    outColor = vec4(fragColor, 1.0);
}
