#version 450
#extension GL_GOOGLE_include_directive : require

#include "util.comp"

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D tex;

layout(set = 1, binding = 0) readonly buffer AtlasMap {
    Texture textures[];
} atlas_map;

layout(set = 7, binding = 0) readonly buffer BlockTextureMap {
    uint textures[];
} block_texture_map;

layout(set = 2, binding = 0) readonly uniform View {
    mat4 camera;
    uvec2 resolution;
    float fov;
} view;

layout(set = 3, binding = 0) readonly uniform ProgramInfo {
    uint frame_number;
    uint start;
} program_info;

layout(set = 4, binding = 0) readonly buffer BrickmapBuffer {
    Brickmap maps[];
} brickmap_buffer;

layout(set = 5, binding = 0) buffer Brickgrid {
    uvec3 size;
    uint _pad;
    uint pointers[];
} brickgrid;

layout(set = 6, binding = 0) readonly buffer TextureBuffer {
    uint textures[];
} block_texture_buffer;

#include "raytracing.comp"

void main() {
    state = uint(gl_FragCoord.x * view.resolution.y + gl_FragCoord.y) + program_info.start + program_info.frame_number;
    vec3 cam_pos = (view.camera * vec4(vec3(0.0), 1.0)).xyz;
    float cam_pos_sum = cam_pos.x + cam_pos.y + cam_pos.z;
    state *= floatBitsToUint(cam_pos_sum) * 648391 + 4535189;

    vec2 frag_coord = vec2(view.resolution.x - gl_FragCoord.x, view.resolution.y - gl_FragCoord.y);
    // frag_coord += vec2(rand_float() - 0.5, rand_float() - 0.5);

    vec2 screen_pos = (frag_coord.xy / vec2(view.resolution)) * 2.0 - 1.0;
    float aspect_ratio = float(view.resolution.x) / float(view.resolution.y);

    float t = tan(view.fov * 0.5 * TO_RADIANS); 
    float px = screen_pos.x * t * aspect_ratio;
    float py = screen_pos.y * t;

    vec3 ro = vec3(0.0);
    vec3 rp = vec3(px, py, 1.0);
    vec3 rd = rp - ro;

    vec3 ray_origin = (view.camera * vec4(ro, 1.0)).xyz;
    vec3 ray_dir = (view.camera * vec4(rd, 0.0)).xyz;
    float rd_sum = ray_dir.x + ray_dir.y + ray_dir.z;
    state *= floatBitsToUint(rd_sum) * 4535189 + 648391;

    Intersection intersection;
    f_color = raymarch_brickgrid(ray_origin, ray_dir, intersection);

    if (!intersection.hit) {
        return;
    }

    // return;

    vec4 hit_color = intersection.raw_color;
    vec3 hit_norm = intersection.normal;
    vec3 gi_ro = intersection.pos + hit_norm * 0.0001;

    const uint GI_SAMPLES = 1;
    vec4 color_add = vec4(0);
    for (int i = 0; i < GI_SAMPLES; i++) {
        vec3 gi_rd = cosine_hemisphere(hit_norm);
        Intersection intersect;
        color_add += raymarch_brickgrid(gi_ro, gi_rd, intersect) * hit_color;
    }

    f_color += color_add / GI_SAMPLES;
}