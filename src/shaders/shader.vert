#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec2 tex_coords;

layout(location = 0) out vec2 tex_out;

layout(push_constant) uniform PushConstantData {
    mat4 camera;
} pc;

void main() {
    gl_Position = pc.camera * vec4(position, 1.0);
    tex_out = tex_coords;
}