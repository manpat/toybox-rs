use crate::gfx;
use std::error::Error;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

#[derive(Copy, Clone, Debug)]
pub struct Shader (pub(super) u32);


struct ImportData {
	source: String,
	resource_scope_id: gfx::ResourceScopeID,
}

struct ShaderCompileResult {
	shader: Shader,
	dependent_resource_scopes: HashSet<gfx::ResourceScopeID>,
}

struct ResolvedShaderSource {
	resolved_source: String,
	dependent_resource_scopes: HashSet<gfx::ResourceScopeID>,
}

type ShaderSourceHash = u64;


struct ShaderCache {
	/// Maps shader hashes to cached compiled shaders.
	source_hash_to_shader: HashMap<ShaderSourceHash, Shader>,

	/// Maps resource scope ids to a list of shader hashes that should be invalidated when the scope is cleaned up.
	scoped_shader_hashes: HashMap<gfx::ResourceScopeID, Vec<ShaderSourceHash>>,
}


pub(super) struct ShaderManager {
	imports: HashMap<String, ImportData>,
	cache: RefCell<ShaderCache>,
}


impl ShaderManager {
	pub fn new() -> Self {
		ShaderManager {
			imports: HashMap::new(),
			cache: ShaderCache {
				source_hash_to_shader: HashMap::new(),
				scoped_shader_hashes: HashMap::new(),
			}.into(),
		}
	}

	pub fn add_import(&mut self, name: impl Into<String>, src: impl Into<String>, resource_scope_id: gfx::ResourceScopeID) {
		use std::collections::hash_map::Entry;

		match self.imports.entry(name.into()) {
			Entry::Occupied(entry) => panic!("Shader import added more than once: {}", entry.key()),
			Entry::Vacant(entry) => {
				entry.insert(ImportData {
					source: src.into(),
					resource_scope_id,
				});
			}
		}
	}

	pub fn get_shader(&self, shaders: &[(u32, &str)]) -> Result<Shader, CompilationError> {
		use std::collections::hash_map::*;
		use std::hash::Hasher;

		let mut hasher = DefaultHasher::new();
		for &(ty, contents) in shaders {
			hasher.write_u32(ty);
			hasher.write(contents.as_bytes());
		}

		let shader_hash = hasher.finish();
		let mut cache_ref = self.cache.borrow_mut();
		let cache = &mut *cache_ref; // reborrow so we can split the borrow

		match cache.source_hash_to_shader.entry(shader_hash) {
			Entry::Occupied(entry) => Ok(*entry.get()),
			Entry::Vacant(entry) => {
				let ShaderCompileResult{shader, dependent_resource_scopes} = compile_shaders(shaders, &self.imports)?;

				for resource_scope in dependent_resource_scopes {
					cache.scoped_shader_hashes.entry(resource_scope)
						.or_insert_with(Vec::new)
						.push(shader_hash);
				}

				Ok(*entry.insert(shader))
			}
		}
	}

	// TODO(pat.m): need a way to ensure shaders can't accidentally import imports from shorter lived resource scopes
	// TODO(pat.m): maybe shaders should work more like other resources, so the ResourceScope itself can handle their deletion?
	// 	would need to figure out how dependencies worked tho, since shaders could reference imports from different scopes.
	pub fn invalidate_shaders_dependent_on_scope(&mut self, resource_scope_id: gfx::ResourceScopeID) {
		let mut cache_ref = self.cache.borrow_mut();
		let cache = &mut *cache_ref; // reborrow so we can split the borrow

		if let Some(hashes) = cache.scoped_shader_hashes.remove(&resource_scope_id) {
			for hash in hashes {
				if let Some(shader) = cache.source_hash_to_shader.remove(&hash) {
					unsafe {
						gfx::raw::DeleteProgram(shader.0);
					}
				}
			}
		}

		self.imports.retain(|_, import_data| import_data.resource_scope_id != resource_scope_id);
	}
}



fn compile_shaders(shaders: &[(u32, &str)], imports: &HashMap<String, ImportData>) -> Result<ShaderCompileResult, CompilationError> {
	use std::ffi::CString;
	use std::str;

	let mut total_dependent_resource_scopes = HashSet::new();

	unsafe {
		let program_handle = gfx::raw::CreateProgram();

		for &(ty, src) in shaders.iter() {
			let ResolvedShaderSource{resolved_source, dependent_resource_scopes} = resolve_imports(&src, imports);
			let src_cstring = CString::new(resolved_source.as_bytes()).unwrap();

			for resource_scope in dependent_resource_scopes {
				total_dependent_resource_scopes.insert(resource_scope);
			}

			let shader_handle = gfx::raw::CreateShader(ty);

			gfx::raw::ShaderSource(shader_handle, 1, &src_cstring.as_ptr(), std::ptr::null());
			gfx::raw::CompileShader(shader_handle);

			let mut status = 0;
			gfx::raw::GetShaderiv(shader_handle, gfx::raw::COMPILE_STATUS, &mut status);

			if status == 0 {
				let mut length = 0;
				gfx::raw::GetShaderiv(shader_handle, gfx::raw::INFO_LOG_LENGTH, &mut length);

				let mut buffer = vec![0u8; length as usize];
				gfx::raw::GetShaderInfoLog(
					shader_handle,
					length,
					std::ptr::null_mut(),
					buffer.as_mut_ptr() as *mut _
				);

				let error = str::from_utf8(&buffer[..buffer.len()-1])
					.map_err(|_| CompilationError::new("shader compilation", "error message invalid utf-8"))?;

				return Err(CompilationError::new("shader compilation", error));
			}

			gfx::raw::AttachShader(program_handle, shader_handle);
			gfx::raw::DeleteShader(shader_handle);
		}

		gfx::raw::LinkProgram(program_handle);

		let mut status = 0;
		gfx::raw::GetProgramiv(program_handle, gfx::raw::LINK_STATUS, &mut status);

		if status == 0 {
			let mut buf = [0u8; 1024];
			let mut len = 0;
			gfx::raw::GetProgramInfoLog(program_handle, buf.len() as _, &mut len, buf.as_mut_ptr() as _);

			let error = str::from_utf8(&buf[..len as usize])
				.map_err(|_| CompilationError::new("shader linking", "error message invalid utf-8"))?;

			return Err(CompilationError::new("shader link", error));
		}

		Ok(ShaderCompileResult {
			shader: Shader(program_handle),
			dependent_resource_scopes: total_dependent_resource_scopes,
		})
	}
}




fn resolve_imports(mut src: &str, imports: &HashMap<String, ImportData>) -> ResolvedShaderSource {
	let search_pattern = "#import";

	let mut resolved_source = String::with_capacity(src.len());
	let mut dependent_resource_scopes = HashSet::new();

	while !src.is_empty() {
		let (prefix, suffix) = match src.split_once(search_pattern) {
			Some(pair) => pair,
			None => {
				resolved_source.push_str(src);
				break
			}
		};

		let (import_name, suffix) = suffix.split_once('\n')
			.expect(&format!("Expected '{search_pattern} <name>'"));

		src = suffix;

		let import_name = import_name.trim();
		let import_data = imports.get(import_name)
			.expect("Unknown import");

		resolved_source.push_str(prefix);
		resolved_source.push_str(&import_data.source);

		dependent_resource_scopes.insert(import_data.resource_scope_id);
	}

	ResolvedShaderSource {
		resolved_source,
		dependent_resource_scopes,
	}
}



#[derive(Debug)]
pub struct CompilationError {
	what: String,
	description: String,
	backtrace: std::backtrace::Backtrace,
}

impl CompilationError {
	fn new(what: &str, description: &str) -> CompilationError {
		CompilationError {
			what: what.into(),
			description: description.into(),
			backtrace: std::backtrace::Backtrace::capture(),
		}
	}
}

impl std::fmt::Display for CompilationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} failed\n", self.what)?;
        write!(f, "{}\n", self.description)?;
        write!(f, "{}\n", self.backtrace)
    }
}


impl Error for CompilationError {}