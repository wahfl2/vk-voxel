#version 450
#extension GL_GOOGLE_include_directive : require

#include "util.comp"

layout(location = 0) out vec4 f_color;

#include "descriptor_sets.comp"

layout(set = 8, binding = 0) readonly buffer SurfelBuffer {
    uint surfel_count;
    Surfel surfels[];
} surfel_buffer;

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
}