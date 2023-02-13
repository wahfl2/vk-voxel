#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 tex_coord;

layout(location = 0) out vec2 tex_out;
layout(location = 1) out vec3 normal_out;

layout(set = 0, binding = 0) uniform sampler2D tex;
layout(push_constant) uniform PushConstantData {
    mat4 camera;
    // Face lighting unneeded
} pc;

void main() {
    gl_Position = pc.camera * vec4(position, 1.0);
    tex_out = tex_coord;
    normal_out = normal;
}