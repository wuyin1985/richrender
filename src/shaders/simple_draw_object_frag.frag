#version 450
#include "rich.cginc"

layout (set = 1, binding = 0) uniform sampler2D texSampler;

layout(location = 0) in vec2 fragTexCoord;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec3 in_camera_dir;

layout(location = 0) out vec4 outColor;

layout (constant_id = 0) const float ambient_strength = 0.2;

void main() {
    vec3 obj_color = vec3(texture(texSampler, fragTexCoord));
    vec3 ambient = ambient_strength * obj_color;

    vec3 light_dir = -ubo.light_dir;

    vec3 normal = normalize(in_normal);
    float diff = max(dot(normal, light_dir), 0.0);
    vec3 diffuse = diff * obj_color;

    vec3 camera_dir = normalize(in_camera_dir);
    vec3 halfway_dir = normalize(light_dir + camera_dir);
    float spec = pow(max(dot(normal, halfway_dir), 0.0), 32.0);
    vec3 specular = spec * vec3(0.3);
    
    vec3 result = (ambient + diffuse + specular);
    outColor = vec4(result, 1.0);
}