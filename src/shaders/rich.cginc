layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 view;
    mat4 proj;
    vec3 light_dir;
    vec3 camera_pos;
} ubo;
