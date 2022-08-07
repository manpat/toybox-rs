use crate::prelude::*;
use crate::gfx::*;

use std::collections::HashMap;
use resource_scope::ResourceScope;


/// The core of the graphics system.
/// Manages the underlying OpenGL context and wraps raw graphics api calls in a safer, higher level api.
///
/// Most operations are gated by 'context' objects:
/// - Resources are constructed through a [`ResourceContext`] object - which is associated with some `ResourceScope`.
///		The point of this to ensure all resources are associated with some scope, so they can be cleaned up at an appropriate time,
///		without introducing too much extra noise or relying too much on RAII.
///		Said object can be acquired with the [`System::resource_context`] call, optionally given some [`ResourceScopeID`].
///
/// - Draw calls and compute dispatches are mediated by a [`DrawContext`] object.
/// 	Said object can be acquired from [`System::draw_context`] at any time (including multiple times per frame),
///		but it is recommended you do this as little as possible to keep draw calls and resource management as
/// 	separate as possible.
pub struct System {
	_sdl_ctx: sdl2::video::GLContext,
	shader_manager: ShaderManager,
	capabilities: Capabilities,
	backbuffer_size: Vec2i,

	pub resources: Resources,

	resource_scope_counter: usize,
	_global_resource_scope_token: ResourceScopeToken,
	resource_scopes: HashMap<ResourceScopeID, ResourceScope>,
}


// Public API
impl System {
	pub fn backbuffer_size(&self) -> Vec2i { self.backbuffer_size }
	pub fn aspect(&self) -> f32 {
		let Vec2{x, y} = self.backbuffer_size.to_vec2();
		x / y
	}

	pub fn capabilities(&self) -> &Capabilities { &self.capabilities }
	
	/// Create a new `ResourceScope` and tie its lifetime to the returned [`ResourceScopeToken`].
	/// The returned token can be passed around and cloned freely. Once no instances of the returned token remain alive,
	/// the [`System`] will destroy all resources that were associated with the resource scope during its lifetime.
	pub fn new_resource_scope(&mut self) -> ResourceScopeToken {
		let resource_scope_id = ResourceScopeID(self.resource_scope_counter);
		self.resource_scope_counter += 1;

		let resource_scope_token = ResourceScopeToken::new(resource_scope_id);
		let resource_scope = ResourceScope::new(resource_scope_token.clone());

		self.resource_scopes.insert(resource_scope_id, resource_scope);

		resource_scope_token
	}

	/// Constructs a temporary [`ResourceContext`] to allow access to resource creation.
	/// If `resource_scope_id` is None, then resources created with the returned context will be
	/// associated with the global resource scope, and won't be destroyed until engine shutdown.
	/// Otherwise if either a [`&ResourceScopeToken`](ResourceScopeToken) or [`ResourceScopeID`] is passed, then resources created with the
	/// returned context will be associated with this resource scope, and will be destroyed when the scope is
	/// cleaned up.
	pub fn resource_context(&mut self, resource_scope_id: impl Into<Option<ResourceScopeID>>) -> ResourceContext<'_> {
		let resource_scope_id = resource_scope_id.into()
			.unwrap_or(ResourceScopeID(0));

		let resource_scope = self.resource_scopes.get_mut(&resource_scope_id)
			.expect("Tried to access already freed scope group");

		ResourceContext {
			resources: &mut self.resources,
			shader_manager: &mut self.shader_manager,
			resource_scope,

			capabilities: &self.capabilities,
			backbuffer_size: self.backbuffer_size,
		}
	}

	/// Constructs a temporary [`DrawContext`] to allow access to draw commands and modifying global pipeline state.
	pub fn draw_context(&mut self) -> DrawContext<'_> {
		DrawContext {
			resources: &self.resources,
			backbuffer_size: self.backbuffer_size,
		}
	}
}

// Internal
impl System {
	pub(crate) fn new(sdl_ctx: sdl2::video::GLContext) -> Self {
		unsafe {
			raw::DebugMessageCallback(Some(gl_message_callback), std::ptr::null());
			raw::Enable(raw::DEBUG_OUTPUT_SYNCHRONOUS);
			raw::Enable(raw::PROGRAM_POINT_SIZE);

			raw::Enable(raw::FRAMEBUFFER_SRGB);

			raw::Enable(raw::DEPTH_TEST);
			// raw::Enable(raw::BLEND);
			// raw::BlendFunc(raw::DST_COLOR, raw::ZERO);
			// raw::BlendEquation(raw::FUNC_ADD);

			raw::Enable(raw::CULL_FACE);
			raw::FrontFace(raw::CCW);
			raw::CullFace(raw::BACK);

			// Disable performance messages
			raw::DebugMessageControl(
				raw::DONT_CARE,
				raw::DEBUG_TYPE_PERFORMANCE,
				raw::DONT_CARE,
				0, std::ptr::null(),
				0 // false
			);

			// Disable notification messages
			raw::DebugMessageControl(
				raw::DONT_CARE,
				raw::DONT_CARE,
				raw::DEBUG_SEVERITY_NOTIFICATION,
				0, std::ptr::null(),
				0 // false
			);
		}

		let global_scope_id = ResourceScopeID(0);
		let global_scope_token = ResourceScopeToken::new(global_scope_id);
		let global_resource_scope = ResourceScope::new(global_scope_token.clone());

		System {
			_sdl_ctx: sdl_ctx,
			shader_manager: ShaderManager::new(),
			capabilities: Capabilities::new(),
			backbuffer_size: Vec2i::splat(1),

			resources: Resources::new(),

			resource_scope_counter: 1,
			_global_resource_scope_token: global_scope_token,
			resource_scopes: [(global_scope_id, global_resource_scope)].into(),
		}
	}

	pub(crate) fn on_resize(&mut self, drawable_size: Vec2i) {
		unsafe {
			raw::Viewport(0, 0, drawable_size.x, drawable_size.y);
		}

		self.backbuffer_size = drawable_size;
		self.resources.on_backbuffer_resize(drawable_size);
	}

	pub(crate) fn cleanup_resources(&mut self) {
		let mut to_remove = Vec::new();

		for (&id, scope) in self.resource_scopes.iter_mut() {
			// If Engine is the sole owner of the resource scope, then noone has any references
			// to it and it should be cleaned up.
			if scope.ref_count() == 1 {
				scope.destroy_owned_resources(&mut self.resources);
				to_remove.push(id);
			}
		}

		for id in to_remove {
			self.resource_scopes.remove(&id);
		}
	}
}


// Not really necessary, but might as well.
impl std::ops::Drop for System {
	fn drop(&mut self) {
		for (_, scope) in self.resource_scopes.iter_mut() {
			scope.destroy_owned_resources(&mut self.resources);
		}
	}
}



extern "system" fn gl_message_callback(source: u32, ty: u32, _id: u32, severity: u32,
	_length: i32, msg: *const i8, _ud: *mut std::ffi::c_void)
{
	let severity_str = match severity {
		raw::DEBUG_SEVERITY_HIGH => "high",
		raw::DEBUG_SEVERITY_MEDIUM => "medium",
		raw::DEBUG_SEVERITY_LOW => "low",
		raw::DEBUG_SEVERITY_NOTIFICATION => return,
		_ => panic!("Unknown severity {}", severity),
	};

	let ty = match ty {
		raw::DEBUG_TYPE_ERROR => "error",
		raw::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "deprecated behaviour",
		raw::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "undefined behaviour",
		raw::DEBUG_TYPE_PORTABILITY => "portability",
		raw::DEBUG_TYPE_PERFORMANCE => "performance",
		raw::DEBUG_TYPE_OTHER => "other",
		_ => panic!("Unknown type {}", ty),
	};

	let source = match source {
		raw::DEBUG_SOURCE_API => "api",
		raw::DEBUG_SOURCE_WINDOW_SYSTEM => "window system",
		raw::DEBUG_SOURCE_SHADER_COMPILER => "shader compiler",
		raw::DEBUG_SOURCE_THIRD_PARTY => "third party",
		raw::DEBUG_SOURCE_APPLICATION => "application",
		raw::DEBUG_SOURCE_OTHER => "other",
		_ => panic!("Unknown source {}", source),
	};

	eprintln!("GL ERROR!");
	eprintln!("Source:   {}", source);
	eprintln!("Severity: {}", severity_str);
	eprintln!("Type:     {}", ty);

	unsafe {
		let msg = std::ffi::CStr::from_ptr(msg as _).to_str().unwrap();
		eprintln!("Message: {}", msg);
	}

	match severity {
		raw::DEBUG_SEVERITY_HIGH | raw::DEBUG_SEVERITY_MEDIUM => panic!("GL ERROR!"),
		_ => {}
	}
}



pub struct IndexedDrawParams {
	pub num_elements: u32,
	pub element_offset: u32,
	pub base_vertex: u32,
}

impl IndexedDrawParams {
	pub fn with_offset(self, element_offset: u32) -> IndexedDrawParams {
		IndexedDrawParams {element_offset, ..self}
	}

	pub fn with_base_vertex(self, base_vertex: u32) -> IndexedDrawParams {
		IndexedDrawParams {base_vertex, ..self}
	}
}

impl<T> From<T> for IndexedDrawParams where T : Into<u32> {
	fn from(num_elements: T) -> IndexedDrawParams {
		IndexedDrawParams {
			num_elements: num_elements.into(),
			element_offset: 0,
			base_vertex: 0,
		}
	}
}

