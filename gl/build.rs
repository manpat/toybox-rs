
use gl_generator::{Registry, Api, Profile, Fallbacks};
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let dest = env::var("OUT_DIR").unwrap();
    let mut file = File::create(&Path::new(&dest).join("gl_bindings.rs")).unwrap();

    println!("cargo::rerun-if-changed=build.rs");
    
	let mut registry = Registry::new(Api::Gl, (4, 6), Profile::Core, Fallbacks::All, &["GL_ARB_parallel_shader_compile"]);

	registry.cmds.retain(should_keep_cmd);

	registry.write_bindings(gl_generator::StructGenerator, &mut file).unwrap();
}

static ALLOWED_GET_FUNCTIONS: &[&str] = &[
	"GetIntegerv",
	"GetInternalformativ",
	"GetObjectLabel",
	"GetProgramBinary",
	"GetProgramInfoLog",
	"GetProgramInterfaceiv",
	"GetProgramiv",
	"GetProgramPipelineInfoLog",
	"GetProgramPipelineiv",
	"GetProgramResourceIndex",
	"GetProgramResourceiv",
	"GetProgramResourceLocation",
	"GetProgramResourceLocationIndex",
	"GetProgramResourceName",
	"GetProgramStageiv",
	"GetQueryBufferObjecti64v",
	"GetQueryBufferObjectiv",
	"GetQueryBufferObjectui64v",
	"GetQueryBufferObjectuiv",
	"GetQueryIndexediv",
	"GetQueryiv",
	"GetQueryObjecti64v",
	"GetQueryObjectiv",
	"GetQueryObjectui64v",
	"GetQueryObjectuiv",
	"GetString",
	"GetStringi",
	"GetSynciv",
	"GetTextureSubImage",
];

static BANNED_PREFIXES: &[&str] = &[
	"Buffer",
	"ClearBuffer",
	"ColorP",
	"CompressedTexImage",
	"CompressedTexSub",
	"CopyTexImage",
	"CopyTexSub",
	"Framebuffer",
	"MapBuffer",
	"MultiTexCoord",
	"NormalP",
	"SecondaryColorP",
	"TexBuffer",
	"TexCoord",
	"TexImage",
	"TexParameter",
	"TexStorage",
	"TexSub",
	"Uniform1",
	"Uniform2",
	"Uniform3",
	"Uniform4",
	"UniformMatrix",
	"VertexAttrib",
	"VertexP",
];

static BANNED_FUNCTIONS: &[&str] = &[
	"ActiveShaderProgram",
	"ActiveTexture",
	"BindRenderbuffer",
	"BindTexture",
	"BindTextures",
	"BindVertexBuffer",
	"BindVertexBuffers",
	"BlitFramebuffer",
	"CheckFramebufferStatus",
	"CopyBufferSubData",
	"DisableVertexAttribArray",
	"DrawBuffer",
	"DrawBuffers",
	"EnableVertexAttribArray",
	"FlushMappedBufferRange",
	"GenBuffers",
	"GenerateMipmap",
	"GenFramebuffers",
	"GenProgramPipelines",
	"GenQueries",
	"GenRenderbuffers",
	"GenSamplers",
	"GenTransformFeedbacks",
	"GenVertexArrays",
	"InvalidateFramebuffer",
	"ReadBuffer",
	"RenderbufferStorage",
	"RenderbufferStorageMultisample",
	"UnmapBuffer",
	"VertexBindingDivisor",
];


fn should_keep_cmd(cmd: &gl_generator::Cmd) -> bool {
	let ident = cmd.proto.ident.as_str();

	if ident.starts_with("Get") {
		return ALLOWED_GET_FUNCTIONS.contains(&ident);
	}

	for prefix in BANNED_PREFIXES {
		if ident.starts_with(prefix) {
			return false
		}
	}

	!BANNED_FUNCTIONS.contains(&ident)
}