#version 450

layout (location = 0) in vec2 position;
layout (location = 1) in vec2 uv;
layout (location = 2) in vec4 color;

layout(std140, row_major, binding = 0) uniform Uniforms {
    mat4 u_transform;
};

out vec2 v_uv;
out vec4 v_color;

// Because imgui only specifies sRGB colors
vec4 srgb_to_linear(vec4 srgb_color) {
    // Calcuation as documented by OpenGL
    vec3 srgb = srgb_color.rgb;
    vec3 selector = ceil(srgb - 0.04045);
    vec3 less_than_branch = srgb / 12.92;
    vec3 greater_than_branch = pow((srgb + 0.055) / 1.055, vec3(2.4));
    return vec4(
        mix(less_than_branch, greater_than_branch, selector),
        srgb_color.a
    );
}

void main() {
    v_uv = uv;
    v_color = srgb_to_linear(color);
    gl_Position = u_transform * vec4(position.xy, 0.0, 1.0);
}

