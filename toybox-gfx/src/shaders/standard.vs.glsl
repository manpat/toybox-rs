

struct Vertex {
	vec3 pos;
	uint uv_packed;
	uvec2 color_packed;
	uvec2 _padding;
};


layout(binding=0) uniform P {
	mat4 u_projection_view;
};

layout(binding=0) readonly buffer V {
	Vertex s_vertices[];
};


out OutVertex {
	vec4 v_color;
	vec2 v_uv;
};

void main() {
	Vertex vertex = s_vertices[gl_VertexID];

	gl_Position = u_projection_view * vec4(vertex.pos.xyz, 1.0);
	gl_PointSize = 6.0;

	v_color = vec4(
		unpackUnorm2x16(vertex.color_packed.x),
		unpackUnorm2x16(vertex.color_packed.y)
	);

	v_uv = unpackUnorm2x16(vertex.uv_packed);
}