in vec4 v_color;
in vec2 v_uv;

layout(binding=0) uniform sampler2D u_texture;

out vec4 o_color;

void main() {
	o_color = texture(u_texture, v_uv).r * v_color;
}