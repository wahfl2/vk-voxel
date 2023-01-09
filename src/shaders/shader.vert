#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec4 color;

layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstantData {
    mat4 camera;
} pc;

void main() {
    gl_Position = pc.camera * vec4(position, 1.0);
    out_color = color;
}