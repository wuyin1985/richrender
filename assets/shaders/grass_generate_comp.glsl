#version 450
#pragma shader_stage(compute)
#define LOCAL_WORK_GROUP_SIZE 32

#define MIN_HEIGHT 0.65
#define MAX_HEIGHT 1.25
#define MIN_WIDTH 0.05
#define MAX_WIDTH 0.07
#define MIN_BEND 7
#define MAX_BEND 13

layout (local_size_x = LOCAL_WORK_GROUP_SIZE, local_size_y = 1, local_size_z = 1) in;

layout(push_constant) uniform PushConsts {
    vec2 grid_size;
    vec2 slot_size;
    uvec2 slot_count;
    float grass_y;
    uint grass_count;
    uint dispatch_size;
} push_constants;

struct GrassBlade {
    vec4 v0;
    vec4 v1;
    vec4 v2;
    vec4 up;
};

layout(std140, binding = 0) buffer GrassBladeBuffer {
    GrassBlade blades[];
};

float rand(vec2 co) {
    return fract(sin(dot(co, vec2(12.9898, 78.233))) * 43758.5453);
}


void main() {
    uint idx = gl_GlobalInvocationID.x;

    uint slot_chess_pos_x = idx % push_constants.slot_count.x;
    uint slot_chess_pos_y = idx / push_constants.slot_count.x;

    vec2 spos = vec2(slot_chess_pos_x * push_constants.slot_size.x, slot_chess_pos_y * push_constants.slot_size.y);
    spos += rand(spos) * push_constants.slot_size;
    float roll = rand(spos);
    float direction = roll * 2.f * 3.14159265f;
    vec4 pos = vec4(spos.x, push_constants.grass_y, spos.y, direction);
    blades[idx].v0 = pos;
    float height = MIN_HEIGHT + roll * (MAX_HEIGHT - MIN_HEIGHT);
    vec3 bladeUp = vec3(0.0, 1.0, 0.0);
    //bezier control point and height
    blades[idx].v1 = vec4(pos.xyz + bladeUp * height, height);
    //physical model guide and width
    float width = MIN_WIDTH + roll * (MAX_WIDTH - MIN_WIDTH);
    blades[idx].v2 = vec4(pos.xyz + bladeUp * height, width);
    //update vector and stiffness
    float stiffness = MIN_BEND + roll * (MAX_BEND - MIN_BEND);
    blades[idx].up = vec4(bladeUp, stiffness);
}
