#version 450
#pragma shader_stage(compute)
#include "rich.cginc"


#define GRAVITY 9.8
#define LOCAL_WORK_GROUP_SIZE 256
layout(local_size_x = LOCAL_WORK_GROUP_SIZE, local_size_y = 1, local_size_z = 1) in;

layout(push_constant) uniform PushConsts {
    vec2 grid_size;
    uvec2 grid_count;
    vec2 slot_size;
    uvec2 slot_count;
    float grass_y;
} push_constants;


struct Blade {
    vec4 v0;
    vec4 v1;
    vec4 v2;
    vec4 up;
};

// input
layout(set = 1, binding = 0) buffer InputBlades {
    Blade inputBlades[];
};

// output
layout(set = 1, binding = 1) buffer CulledBlades {
    Blade culledBlades[];
};

layout(set = 1, binding = 2) buffer NumBlades {
    uint vertexCount;
    uint instanceCount;
    uint firstVertex;
    uint firstInstance;
} numBlades;

bool inBounds(float value, float bounds) {
    return (value >= -bounds) && (value <= bounds);
}

float rand(vec2 co) {
    return fract(sin(dot(co, vec2(12.9898, 78.233))) * 43758.5453);
}

void update_blade(uint index) {
    Blade blade = inputBlades[index];
    // blade parameters
    vec3 v0 = blade.v0.xyz;
    vec3 v1 = blade.v1.xyz;
    vec3 v2 = blade.v2.xyz;
    vec3 up = blade.up.xyz;

    float angle = blade.v0.w;// orientation of the blade of grass;
    // if this angle is zero the thin width portion (non-flat portion) of the grass is facing +ve x.
    float height = blade.v1.w;
    float width = blade.v2.w;
    float stiffness = blade.up.w;

    vec3 side_direction = vec3(cos(angle), 0.0, sin(angle));
    vec3 front_direction = normalize(cross(up, side_direction));

    // Apply Natural Forces on every blade
    // Recovery Force
    vec3 initial_v2 = v0 + up * height;
    vec3 recovery = (initial_v2 - v2)*stiffness;

    // Gravity Force
    vec3 gE = vec3(0.0, -GRAVITY, 0.0);//Environmental Gravity
    vec3 gF = 0.25 * GRAVITY * front_direction;//Front Gravity
    vec3 gravity = gE + gF;//Total Gravitational Force

    // Wind Force
    vec3 windDirection = normalize(vec3(1, 1, 1));// straight wave

    float windStrength = 10.0 * rand(v0.xz) * cos(ubo.totalTime);

    float fd = 1.0 - abs(dot(windDirection, normalize(v2 - v0)));
    float fr = dot(v2 - v0, up) / height;
    float theta = fd * fr;

    vec3 wind = windStrength * windDirection * theta;

    // Resulting Translation due to forces over delta time
    vec3 translation_dt = (recovery + wind + gravity) * ubo.deltaTime;

    v2 += translation_dt;

    // State Validation
    // 1. v2 has to remain above the local plane
    v2 -= up * min(dot(up, (v2-v0)), 0.0);

    // 2. grass blade always has a slight curvature
    vec3 l_proj = abs(v2-v0 - up * dot((v2-v0), up));
    v1 = v0 + height*up * max((1.0 - l_proj)/height, 0.05*max((l_proj/height), 1.0));

    // 3. length of Bezier not larger than blade height
    float L0 = distance(v0, v2);
    float L1 = distance(v0, v1) + distance(v1, v2);
    float n = 2.0;
    float L = (2.0 * L0 + (n - 1.0) * L1) / (n + 1.0);
    float r = height / L;

    // Corrected Values of v1 and v2
    vec3 v1_corrected = v0 + r*(v1-v0);
    vec3 v2_corrected = v1_corrected + r*(v2-v1);

    // Update the input blades so the state propogates
    inputBlades[index].v1.xyz = v1_corrected;
    inputBlades[index].v2.xyz = v2_corrected;

    // Cull Blades
    // 1. Orientation culling
    mat4 inverseViewMat = inverse(ubo.view);
    vec3 eye_worldSpace = (inverseViewMat * vec4(0.0, 0.0, 0.0, 1.0)).xyz;
    vec3 viewDirection = eye_worldSpace - v0;
    bool culled_Due_To_Orientaion = dot(viewDirection, front_direction) > 0.8;

    // 2. View-frustum culling
    float tolerance = 3.0f;

    vec4 v0_NDC = ubo.proj * ubo.view * vec4(v0, 1.0);
//    culled_Due_To_Frustum = (!inBounds(v0_NDC.x, v0_NDC.w + tolerance) ||!inBounds(v0_NDC.y, v0_NDC.w + tolerance));
//
//    if (culled_Due_To_Frustum)
//    {
//        vec3 m = 0.25 * v0 + 0.5 * v1 + 0.25 * v2;
//        vec4 m_NDC = ubo.proj * ubo.view * vec4(m, 1.0);
//        culled_Due_To_Frustum = (!inBounds(m_NDC.x, m_NDC.w + tolerance) ||!inBounds(m_NDC.y, m_NDC.w + tolerance));
//    }
//
//    if (culled_Due_To_Frustum)
//    {
//        vec4 v2_NDC = ubo.proj * ubo.view * vec4(v2, 1.0);
//        culled_Due_To_Frustum = (!inBounds(v2_NDC.x, v2_NDC.w + tolerance) ||!inBounds(v2_NDC.y, v2_NDC.w + tolerance));
//    }

    // 3. Distance culling
    float projected_distance = length(v0 - eye_worldSpace - up * dot(up, (v0 - eye_worldSpace)));
    float dmax = 40.0;
    float numBuckets = 10.0;
    bool culled_Due_To_Distance = mod(index, numBuckets) > floor(numBuckets * (1.0 - projected_distance/dmax));

    // Atomic operation to read and update numBlades.vertexCount is required because the compute shader is
    // parallezied over the number of grass blades, ie two threads could try to update the numBlades.vertexCount
    // at the same time.
    // You want to write the visible blades to the buffer without write conflicts between threads

    //if (!culled_Due_To_Distance && !culled_Due_To_Frustum && !culled_Due_To_Orientaion)//
    {
        //culledBlades[atomicAdd(numBlades.vertexCount, 1)] = inputBlades[index];
        culledBlades[index] = inputBlades[index];
    }
}

void main()
{
    // Reset the number of blades to 0
    if (gl_GlobalInvocationID.x == 0)
    {
        numBlades.vertexCount = 0;
    }
    barrier();// Wait till all threads reach this point

    uint slot_count_u = push_constants.slot_count.x * push_constants.slot_count.y;
    uint start_idx = (gl_WorkGroupID.x * gl_WorkGroupID.y) * slot_count_u;
    uint draw_count = uint(ceil(float(slot_count_u)/LOCAL_WORK_GROUP_SIZE));

    for (uint i = 0; i < draw_count; i++) {
        uint idx = gl_LocalInvocationIndex + i + start_idx;
        update_blade(idx);
    }
}