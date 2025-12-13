
in Vertex {
	vec4 v_color;
	vec2 v_uv;
};

out vec4 o_color;

void main() {
	o_color = v_color;
}