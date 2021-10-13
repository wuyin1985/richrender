layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 view;
    mat4 proj;
    mat4 light_matrix;
    vec4 light_dir;
    vec4 camera_pos;
    float deltaTime;
    float totalTime;
} ubo;


