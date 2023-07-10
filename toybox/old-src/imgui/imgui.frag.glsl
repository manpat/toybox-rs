#version 450

in vec2 v_uv;
in vec4 v_color;

layout(binding = 0) uniform sampler2D u_tex;
layout(location = 0) out vec4 out_color;

void main() {
    vec4 linear_color = v_color * texture(u_tex, v_uv.st);
    out_color = linear_color;
}