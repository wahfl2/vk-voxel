#version 460

struct BlockQuad {
    vec3 position;
    uint face_tex;
};

layout(std140, set = 3, binding = 0) readonly buffer BlockBuffer {
    BlockQuad quads[];
} block_buffer;

layout(std140, set = 4, binding = 0) readonly buffer AtlasMap {
    vec4 textures[];
} atlas_map;

layout(set = 1, binding = 0) uniform View {
    mat4 camera;
} view;

layout(location = 0) out vec2 tex_out;
layout(location = 1) out vec3 normal;

const uint BIT_MASK_29 = 536870911;
const float HALF = 0.5;
const uint[6] VERT_INDICES = uint[](0, 1, 2, 0, 2, 3);
const vec3[6] NORMALS = vec3[](
    vec3( 1.0, 0.0, 0.0),
    vec3(-1.0, 0.0, 0.0),
    vec3(0.0,  1.0, 0.0),
    vec3(0.0, -1.0, 0.0),
    vec3(0.0, 0.0,  1.0),
    vec3(0.0, 0.0, -1.0)
);
const vec3[6][4] VERT_OFFSETS = vec3[][](
    // +X face
    vec3[](
        vec3(0.0, -HALF, -HALF), 
        vec3(0.0,  HALF, -HALF), 
        vec3(0.0,  HALF,  HALF), 
        vec3(0.0, -HALF,  HALF)
    ),
    // -X face
    vec3[](
        vec3(0.0, -HALF,  HALF),
        vec3(0.0,  HALF,  HALF), 
        vec3(0.0,  HALF, -HALF), 
        vec3(0.0, -HALF, -HALF)
    ),
    // +Y face
    vec3[](
        vec3( HALF, 0.0,  HALF), 
        vec3( HALF, 0.0, -HALF),
        vec3(-HALF, 0.0, -HALF), 
        vec3(-HALF, 0.0,  HALF)
    ),
    // -Y face
    vec3[](
        vec3(-HALF, 0.0,  HALF),
        vec3(-HALF, 0.0, -HALF), 
        vec3( HALF, 0.0, -HALF),
        vec3( HALF, 0.0,  HALF)
    ),
    // +Z face
    vec3[](
        vec3( HALF, -HALF, 0.0),
        vec3( HALF,  HALF, 0.0), 
        vec3(-HALF,  HALF, 0.0),
        vec3(-HALF, -HALF, 0.0)
    ),
    // -Z face
    vec3[](
        vec3(-HALF, -HALF, 0.0),
        vec3(-HALF,  HALF, 0.0),
        vec3( HALF,  HALF, 0.0), 
        vec3( HALF, -HALF, 0.0)
    )
);

void main() {
    const uint quad_index = gl_VertexIndex / 6;
    const BlockQuad quad = block_buffer.quads[quad_index];
    
    const vec4 tex = atlas_map.textures[quad.face_tex & BIT_MASK_29];
    const uint face = quad.face_tex >> 29;

    const vec2 tex_max_min[4] = vec2[](tex.zw, tex.zy, tex.xy, tex.xw);

    const uint vert_idx = VERT_INDICES[gl_VertexIndex % 6];
    const vec3 vert_pos = VERT_OFFSETS[face][vert_idx] + quad.position;

    gl_Position = view.camera * vec4(vert_pos, 1.0);
    tex_out = tex_max_min[vert_idx];
    normal = NORMALS[face];
}