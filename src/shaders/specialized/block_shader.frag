#version 450

const float PI = 3.1415926535897932384626;
const float TO_RADIANS = PI / 180.0;
const int MAX_RAY_STEPS = 128;
const int MAX_INNER_STEPS = 32;
const uvec3 SECTION_SIZE = uvec3(8, 8, 8);
const float RECIP_255 = 0.00392156862;

const float DEG_90 = 90.0 * TO_RADIANS;
const mat2 ROT_90 = mat2(cos(DEG_90), -sin(DEG_90), sin(DEG_90), cos(DEG_90));

struct Brickmap {
    uint solid_mask[16];
    uint textures_offset;
    uint lod_color;
};

struct Texture {
    uint offset_xy;
    uint size_xy;
    uvec2 _pad;
};

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D tex;

layout(std140, set = 1, binding = 0) readonly buffer AtlasMap {
    Texture textures[];
} atlas_map;

layout(set = 7, binding = 0) readonly buffer BlockTextureMap {
    uint textures[];
} block_texture_map;

layout(set = 2, binding = 0) uniform View {
    mat4 camera;
    uvec2 resolution;
    float fov;
} view;

layout(set = 3, binding = 0) uniform FaceLighting {
    vec3 positive;
    uint _pad1;
    vec3 negative;
    uint _pad2;
} face_lighting;

layout(set = 4, binding = 0) readonly buffer BrickmapBuffer {
    Brickmap maps[];
} brickmap_buffer;

layout(set = 5, binding = 0) readonly buffer Brickgrid {
    uvec3 size;
    uint _pad;
    uint pointers[];
} brickgrid;

layout(set = 6, binding = 0) readonly buffer TextureBuffer {
    uint textures[];
} block_texture_buffer;

bvec3 and_bvec(bvec3 n, bvec3 m) {
    return bvec3(n.x && m.x, n.y && m.y, n.z && m.z);
}

ivec2 get_texel(Texture texture_s, vec2 uv) {
    uvec2 offset = uvec2(texture_s.offset_xy & 65535, texture_s.offset_xy >> 16);
    uvec2 size = uvec2(texture_s.size_xy & 65535, texture_s.size_xy >> 16);
    return ivec2(offset + (size * uv));
}

// Used for obtaining the texture index of a full block
uint count_full_preceding(uint solid_mask[16], ivec3 section_pos) {
    const uint CHUNK_HEIGHT = brickgrid.size.y;

    uint bit_index = (section_pos.x * SECTION_SIZE.y * SECTION_SIZE.z) + (section_pos.y * SECTION_SIZE.z) + section_pos.z;
    uint t_idx = 0;
    for (uint i = 0; i < bit_index; i++) {
        uint n = solid_mask[i / 32];
        uint inner_idx = i % CHUNK_HEIGHT;
        t_idx += (n >> inner_idx) & 1;
    }
    return t_idx;
}

bool index_map(uint solid_mask[16], ivec3 section_pos, out bool out_of_range) {
    const uint CHUNK_HEIGHT = brickgrid.size.z;

    if (any(greaterThanEqual(section_pos, ivec3(SECTION_SIZE))) || any(lessThan(section_pos, ivec3(0)))) {
        out_of_range = true;
        return false;
    } else {
        out_of_range = false;
    }

    uint bit_index = (section_pos.x * SECTION_SIZE.y * SECTION_SIZE.z) + (section_pos.y * SECTION_SIZE.z) + section_pos.z;

    uint n = solid_mask[bit_index / 32];
    uint inner_idx = bit_index % CHUNK_HEIGHT;
    return bool((n >> inner_idx) & 1);
}

uint index_grid(ivec3 grid_pos, out bool out_of_range) {
    if (grid_pos.y < 0 || grid_pos.y >= brickgrid.size.y) {
        out_of_range = true;
        return 0;
    } else {
        out_of_range = false;
    }

    uvec3 idx_3d = uvec3(mod(grid_pos, ivec3(brickgrid.size)));
    uint idx = (idx_3d.x * brickgrid.size.y * brickgrid.size.z) + (idx_3d.y * brickgrid.size.z) + idx_3d.z;
    return brickgrid.pointers[idx];
}

void normal_shading(in vec3 n, out float ret) {
    vec3 v = ((max(n, vec3(0.0)) * face_lighting.positive) + (-1.0 * min(n, vec3(0.0)) * face_lighting.negative)) * abs(n);
    ret = v.x + v.y + v.z;
}

void main() {
    // vec4 placeholder1 = texture(tex, vec2(0.0));
    // vec4 placeholder2 = atlas_map.textures[0];
    uint placeholder3 = face_lighting._pad1;

    vec2 frag_coord = vec2(view.resolution.x - gl_FragCoord.x, view.resolution.y - gl_FragCoord.y);
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

    vec3 grid_ray_origin = ray_origin / vec3(SECTION_SIZE);
    ivec3 grid_pos = ivec3(floor(grid_ray_origin));

    vec3 norm_ray_dir = normalize(ray_dir);

    vec3 delta_dist = abs(1.0 / norm_ray_dir);

    ivec3 ray_step = ivec3(sign(ray_dir));
    vec3 side_dist = (sign(ray_dir) * (vec3(grid_pos) - grid_ray_origin) + (sign(ray_dir) * 0.5) + vec3(0.5)) * delta_dist;
    bvec3 grid_mask = bvec3(false);

    for (int i = 0; i < MAX_RAY_STEPS; i++) {
        bool out_of_range = false;
        uint ptr = index_grid(grid_pos, out_of_range);
        if (out_of_range) {
            discard;
        };

        uint flags = ptr & 3;
        uint data = ptr >> 2;

        if (flags == 1) {
            // Empty brickmap, continue

            grid_mask = lessThanEqual(side_dist.xyz, min(side_dist.yzx, side_dist.zxy));
            side_dist += vec3(grid_mask) * delta_dist;
            grid_pos += ivec3(vec3(grid_mask)) * ray_step;
        } else if (flags == 0) {
            // Unloaded brickmap

            // feedback?
            discard;
        } else if (flags == 2) {
            // LOD brickmap
            f_color = vec4(
                float((data >>  0) & 255) * RECIP_255,
                float((data >>  8) & 255) * RECIP_255,
                float((data >> 16) & 255) * RECIP_255,
                1.0
            );
            // also feedback?
            return;
        } else {
            // Brickmap 

            Brickmap brickmap = brickmap_buffer.maps[data];
            float d = length(vec3(grid_mask) * (side_dist - delta_dist)) / length(ray_dir);

            vec3 grid_intersect = grid_ray_origin + (d * ray_dir);
            vec3 intersect = grid_intersect * vec3(SECTION_SIZE);

            // This assumes square sections.
            vec3 off = sign(ray_dir) * vec3(grid_mask) * 0.5;
            ivec3 block_pos = ivec3(floor(intersect + off));
            ivec3 section_pos = ivec3(mod(block_pos, SECTION_SIZE));

            vec3 side_dist_sec = (sign(ray_dir) * (block_pos - intersect) + (sign(ray_dir) * 0.5) + vec3(0.5)) * delta_dist;

            bvec3 mask = grid_mask;
            bool out_of_range = false;

            for (int j = 0; j < MAX_INNER_STEPS; j++) {
                bool solid = index_map(brickmap.solid_mask, section_pos, out_of_range);

                if (out_of_range || solid) {
                    break;
                }

                mask = lessThanEqual(side_dist_sec.xyz, min(side_dist_sec.yzx, side_dist_sec.zxy));
                side_dist_sec += vec3(mask) * delta_dist;
                section_pos += ivec3(vec3(mask)) * ray_step;
            }

            if (!out_of_range) {
                uint negative = uint(any(and_bvec(mask, greaterThanEqual(ray_dir, vec3(0)))));
                uvec3 u_mask = uvec3(mask);
                uint face_id = (u_mask.y * 2u) + (u_mask.z * 4u) + negative;
                uint face_axis = u_mask.y + (u_mask.z * 2u);

                float d_sec = length(vec3(mask) * (side_dist_sec - delta_dist)) / length(ray_dir);

                vec3 sec_intersect = intersect + d_sec * ray_dir;
                vec3 sd = sec_intersect - floor(sec_intersect);

                vec2 yz = ROT_90 * (sd.yz - 0.5) + 0.5;

                vec2 possible_uv[3] = vec2[3](
                    yz,
                    sd.xz,
                    1.0 - sd.xy
                );

                vec2 uv = possible_uv[face_axis];

                uint block_texture_index = brickmap.textures_offset + count_full_preceding(brickmap.solid_mask, section_pos);
                uint block_texture_id = block_texture_buffer.textures[block_texture_index];
                uint texture_index = block_texture_map.textures[block_texture_id * 6 + face_id];
                
                Texture texture_s = atlas_map.textures[texture_index];

                f_color = texelFetch(tex, get_texel(texture_s, uv), 0);
                return;
            } else {
                grid_mask = lessThanEqual(side_dist.xyz, min(side_dist.yzx, side_dist.zxy));
                side_dist += vec3(grid_mask) * delta_dist;
                grid_pos += ivec3(vec3(grid_mask)) * ray_step;
            }
        }
    }

    f_color = vec4(1.0, 0.0, 1.0, 0.0);
}