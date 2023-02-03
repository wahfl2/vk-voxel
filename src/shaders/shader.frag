#version 450

layout(location = 0) in vec2 tex_out;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec4 f_color;

struct FaceLighting {
    vec3 positive;
    uint _pad1;
    vec3 negative;
    uint _pad2;
};

layout(set = 0, binding = 0) uniform sampler2D tex;
layout(push_constant) uniform PushConstantData {
    mat4 _;
    FaceLighting face_lighting;
} pc;

void normal_shading(in vec3 n, out float ret) {
    vec3 v = ((max(n, vec3(0.0)) * pc.face_lighting.positive) + (-1.0 * min(n, vec3(0.0)) * pc.face_lighting.negative)) * abs(n);
    ret = v.x + v.y + v.z;
}

void main() {
    vec3 tex_color = texture(tex, tex_out).xyz;
    float shading;
    normal_shading(normal, shading);
    f_color = vec4(tex_color * shading, 1.0);
}