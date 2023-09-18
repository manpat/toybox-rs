

struct Vertex {
	vec2 pos;	
	uint uv;	
	uint color;

	int clip_lr;
	int clip_tb;
};


layout(binding=0) readonly buffer V {
	Vertex s_vertices[];
};

layout(binding=0) uniform T {
	ivec2 u_screen_size;
};


out vec4 v_color;
out vec2 v_uv;


void main() {
	Vertex vertex = s_vertices[gl_VertexID];

	vec2 pos = vertex.pos / vec2(u_screen_size) * 2.0 - 1.0;
	gl_Position = vec4(pos.x, -pos.y, 0.0, 1.0);

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


	float clip_left = float(bitfieldExtract(vertex.clip_lr, 0, 16));
	float clip_right = float(bitfieldExtract(vertex.clip_lr, 16, 16));
	float clip_top = float(bitfieldExtract(vertex.clip_tb, 0, 16));
	float clip_bottom = float(bitfieldExtract(vertex.clip_tb, 16, 16));

	// NOTE: egui is y-down. these are all in UI space
	gl_ClipDistance[0] = vertex.pos.x - clip_left;
	gl_ClipDistance[1] = vertex.pos.y - clip_top;
	gl_ClipDistance[2] = clip_right - vertex.pos.x;
	gl_ClipDistance[3] = clip_bottom - vertex.pos.y;
}