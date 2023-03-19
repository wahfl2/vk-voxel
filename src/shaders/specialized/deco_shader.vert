#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 tex_coord;

layout(location = 0) out vec2 tex_out;
layout(location = 1) out vec3 normal_out;

layout(set = 1, binding = 0) uniform View {
    mat4 camera;
} view;

void main() {
    gl_Position = view.camera * vec4(position, 1.0);
    tex_out = tex_coord;
    normal_out = normal;
}