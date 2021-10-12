#version 450
#pragma shader_stage(compute)
#define LOCAL_WORK_GROUP_SIZE 256

layout (local_size_x = LOCAL_WORK_GROUP_SIZE, local_size_y = 1, local_size_z = 1) in;

layout(push_constant) uniform PushConsts {
    vec2 grid_size;
    uvec2 grid_count;
    vec2 slot_size;
    uvec2 slot_count;
    float grass_y;
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


void main() {

    uint slot_count_u = push_constants.slot_count.x * push_constants.slot_count.y;
    uint draw_count = uint(floor(slot_count_u/LOCAL_WORK_GROUP_SIZE));
    uint start_idx = (gl_WorkGroupID.x * gl_WorkGroupID.y) * slot_count_u;

    vec2 grid_pos = gl_WorkGroupID.xy * push_constants.grid_size;
    for (int i = 0; i < draw_count;i++) {
        uint idx = gl_LocalInvocationIndex + i + start_idx;
        uint slot_chess_pos_x = idx % push_constants.slot_count.x;
        uint slot_chess_pos_y = idx / push_constants.slot_count.x;

        vec2 spos = vec2(slot_chess_pos_x * push_constants.slot_size.x,
        slot_chess_pos_y * push_constants.slot_size.y);

        spos += grid_pos;

        vec4 pos = vec4(spos.x, push_constants.grass_y, spos.y, 0.0);
        blades[idx].v0 = pos;
        blades[idx].v1 = vec4(0.0, 3.0, 0.0, 5.0);
        blades[idx].v2 = vec4(1.4, 4.0, 0.0, 2.0);
        blades[idx].up = vec4(0.0, 1.0, 0.0, 1.0);
    }
}
