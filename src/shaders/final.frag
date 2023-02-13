#version 450

layout(input_attachment_index = 0, set = 2, binding = 0) uniform subpassInput u_blocks;
layout(input_attachment_index = 1, set = 2, binding = 1) uniform subpassInput u_decorations;
layout(input_attachment_index = 2, set = 2, binding = 2) uniform subpassInput u_depth_blocks;
layout(input_attachment_index = 3, set = 2, binding = 3) uniform subpassInput u_depth_decorations;

layout(location = 0) out vec4 f_color;

void main() {
    float b_depth = subpassLoad(u_depth_blocks).x;
    float d_depth = subpassLoad(u_depth_decorations).x;

    if (b_depth + d_depth >= 2.0) {
        discard;
    }

    vec3 col;
    if (b_depth > d_depth) {
        col = subpassLoad(u_decorations).xyz;
    } else {
        col = subpassLoad(u_blocks).xyz;
    }

    f_color = vec4(col, 1.0);
}