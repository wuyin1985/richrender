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
    float shadow = 1.0;
    if ( shadowCoord.z > -1.0 && shadowCoord.z < 1.0 )
    {
        float dist = texture( shadow_map, shadowCoord.st + off ).r;
        if ( shadowCoord.w > 0.0 && dist < shadowCoord.z )
        {
            shadow = ambient_strength;
        }
    }
    return shadow;
    //return texture( shadow_map, shadowCoord.st + off ).r;
}


#ifdef IN_TEX_COORD
vec3 draw_light() {
    vec3 obj_color = vec3(texture(tex_sample, in_tex_coord));
#ifdef IN_NORMAL
    vec3 ambient = ambient_strength * obj_color;
    vec3 light_dir = -ubo.light_dir;
    vec3 normal = normalize(in_normal);
    float diff = max(dot(normal, light_dir), 0.0);
    vec3 diffuse = diff * obj_color;

    vec3 camera_dir = normalize(in_camera_dir);
    vec3 halfway_dir = normalize(light_dir + camera_dir);
    float spec = pow(max(dot(normal, halfway_dir), 0.0), 32.0);
    vec3 specular = spec * vec3(0.3);
    vec3 result = diffuse + specular + ambient;
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
    //oc = vec3(1.0, 1.0, 1.0);
    float shadow = textureProj(in_shadow_coord / in_shadow_coord.w, vec2(0.0));
    out_color = vec4(oc * shadow, 1.0);
}