layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 view;
    mat4 proj;
    mat4 light_matrix;
    vec4 light_dir;
    vec4 camera_pos;
    float deltaTime;
    float totalTime;
} ubo;

const mat4 shadowBiasMat = mat4(
0.5, 0.0, 0.0, 0.0,
0.0, -0.5, 0.0, 0.0,
0.0, 0.0, 1.0, 0.0,
0.5, 0.5, 0.0, 1.0 );
