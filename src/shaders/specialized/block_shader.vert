#version 460

struct BlockQuad {
    vec3 position;
    uint face;
    vec4 tex;
};

layout(std140, set = 1, binding = 0) readonly buffer ObjectBuffer {
    BlockQuad quads[];
} block_buffer;

layout(location = 0) out vec2 tex_out;
layout(location = 1) out vec3 normal;

layout(push_constant) uniform PushConstantData {
    mat4 camera;
} pc;

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
    const vec2 tex_max_min[4] = vec2[](quad.tex.zw, quad.tex.zy, quad.tex.xy, quad.tex.xw);

    const uint vert_idx = VERT_INDICES[gl_VertexIndex % 6];
    const vec3 vert_pos = VERT_OFFSETS[quad.face][vert_idx] + quad.position;

    gl_Position = pc.camera * vec4(vert_pos, 1.0);
    tex_out = tex_max_min[vert_idx];
    normal = NORMALS[quad.face];
}