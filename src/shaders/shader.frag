#version 450

layout(location = 0) in vec2 tex_out;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D tex;

void main() {
    vec3 color = texture(tex, tex_out).xyz;
    f_color = vec4(color * normal, 1.0);
}