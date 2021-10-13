#version 450
#pragma shader_stage(fragment)
#include "rich.cginc"

#ifdef IN_TEX_COORD
layout (set = 1, binding = 0) uniform sampler2D tex_sample;
layout(location = 0) in vec2 in_tex_coord;
#endif

layout (set = 1, binding = 1) uniform sampler2D shadow_map;

layout(location = 1) in vec4 in_shadow_coord;

layout(location = 2) in vec3 in_camera_dir;

#ifdef IN_NORMAL
layout(location = 3) in vec3 in_normal;
#endif

layout(location = 0) out vec4 out_color;

layout (constant_id = 0) const float ambient_strength = 0.3;

#define ambient_shadow 0.1



float textureProj(vec4 shadowCoord, vec2 off)
{
    float shadow = 0.0;
    float dist = texture(shadow_map, shadowCoord.xy + off).r;
    if (shadowCoord.w > 0 && dist < shadowCoord.z)
    {
        shadow = 1.0;
    }
    return shadow;
    //    shadowCoord.y = -shadowCoord.y;
    //    vec2 uv = shadowCoord.st * 0.5 + 0.5;
    //    return texture(shadow_map, uv).r;
}

float filterPCF(vec4 sc)
{
    ivec2 texDim = textureSize(shadow_map, 0);
    float scale = 1.5;
    float dx = scale * 1.0 / float(texDim.x);
    float dy = scale * 1.0 / float(texDim.y);

    float shadowFactor = 0.0;
    int count = 0;
    int range = 1;

    for (int x = -range; x <= range; x++)
    {
        for (int y = -range; y <= range; y++)
        {
            shadowFactor += textureProj(sc, vec2(dx*x, dy*y));
            count++;
        }

    }
    return shadowFactor / count;
}


    #ifdef IN_TEX_COORD
vec3 draw_light() {
    vec3 obj_color = vec3(texture(tex_sample, in_tex_coord));
    #ifdef IN_NORMAL
    vec3 ambient = ambient_strength * obj_color;
    vec3 light_dir = -ubo.light_dir.xyz;
    vec3 normal = normalize(in_normal);
    float diff = max(dot(normal, light_dir), 0.0);
    vec3 diffuse = diff * obj_color;

    vec3 camera_dir = normalize(in_camera_dir);
    vec3 halfway_dir = normalize(light_dir + camera_dir);
    float spec = pow(max(dot(normal, halfway_dir), 0.0), 32.0);
    vec3 specular = spec * vec3(0.3);

    float shadow = filterPCF(in_shadow_coord / in_shadow_coord.w);
    vec3 result = ambient + (1.0 - shadow) * (specular + diffuse);
    return result;
    #else
    return obj_color;
    #endif
}
    #endif

void main() {
    vec3 oc;
    #ifdef IN_TEX_COORD
    oc = draw_light();
    #else
    oc = vec3(1.0, 1.0, 1.0);
    #endif
    out_color = vec4(oc, 1.0);
}