

struct Vertex {
	vec2 pos;	
	uint uv;	
	uint color;
};


layout(binding=0) readonly buffer V {
	Vertex s_vertices[];
};


out vec4 v_color;
out vec2 v_uv;


void main() {
	Vertex vertex = s_vertices[gl_VertexID];
	gl_Position = vec4(vertex.pos.x, -vertex.pos.y, 0.0, 1.0);

	v_color = vec4(
		float(bitfieldExtract(vertex.color, 0, 8)) / 255.0,
		float(bitfieldExtract(vertex.color, 8, 8)) / 255.0,
		float(bitfieldExtract(vertex.color, 16, 8)) / 255.0,
		float(bitfieldExtract(vertex.color, 24, 8)) / 255.0
	);

	v_uv = vec2(
		float(bitfieldExtract(vertex.uv, 0, 16)) / 65535.0,
		float(bitfieldExtract(vertex.uv, 16, 16)) / 65535.0
	);
}