use crate::prelude::*;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ShaderName {
	pub raw: u32,
	pub shader_type: ShaderType,
}

impl super::ResourceName for ShaderName {
	const GL_IDENTIFIER: u32 = gl::PROGRAM;
	fn as_raw(&self) -> u32 { self.raw }
}


#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum ShaderType {
	Vertex = gl::VERTEX_SHADER,
	Fragment = gl::FRAGMENT_SHADER,
	Compute = gl::COMPUTE_SHADER,
}


/// Shaders
impl super::Core {
	pub fn create_shader(&self, shader_type: ShaderType, src_chunks: &[&str]) -> anyhow::Result<ShaderName> {
		use std::ffi::CString;

		let c_strings: Vec<_> = src_chunks.iter()
			.map(|s| {
				let mut v = s.as_bytes().to_owned();
				v.push(b'\n'); // Things can go wrong if a chunk doesn't end in a newline
				CString::new(v)
			})
			.collect::<Result<_, _>>()?;

		let c_string_ptrs: Vec<_> = c_strings.iter().map(|s| s.as_ptr()).collect();

		let program_name = unsafe {
			self.gl.CreateShaderProgramv(shader_type as u32, c_string_ptrs.len() as _, c_string_ptrs.as_ptr())
		};

		if program_name == 0 {
			anyhow::bail!("Failed to compile shader")
		}


		let mut status = 0;
		unsafe {
			self.gl.GetProgramiv(program_name, gl::LINK_STATUS, &mut status);
		}

		if status == 0 {
			let mut buf = [0u8; 1024];
			let mut len = 0;

			unsafe {
				self.gl.GetProgramInfoLog(program_name, buf.len() as _, &mut len, buf.as_mut_ptr() as _);
				self.gl.DeleteProgram(program_name);
			}

			let error = std::str::from_utf8(&buf[..len as usize])?;
			anyhow::bail!("{error}");
		}

		Ok(ShaderName {
			raw: program_name,
			shader_type,
		})
	}

	pub fn destroy_shader(&self, name: ShaderName) {
		unsafe {
			self.gl.DeleteProgram(name.raw)
		}
	}
}
