//! Higher level mesh building and management.
//!
//! The core of this module is the [`Mesh`] and [`BasicMesh`] types, and the generic poly builder apis
//! defined by [`PolyBuilder2D`] and [`PolyBuilder3D`].

use crate::prelude::*;

pub mod traits;
pub mod util;
pub mod geom_2d;
pub mod geom_3d;
pub mod color_mesh_builder;
pub mod plane_mesh_builder_adaptor;

#[doc(inline)] pub use util::*;
#[doc(inline)] pub use traits::{PolyBuilder2D, PolyBuilder3D};
#[doc(inline)] pub use color_mesh_builder::ColorMeshBuilder;
#[doc(inline)] pub use plane_mesh_builder_adaptor::{PlaneMeshBuilderAdaptor, BuilderSurface, OrthogonalOrientation};

/// Reexports [geom_2d] and [geom_3d] modules for convenience.
pub mod geom {
	#[doc(inline)] pub use super::geom_2d::*;
	#[doc(inline)] pub use super::geom_3d::*;
}


/// Aggregates a [`gfx::Vao`], vertex [`gfx::Buffer`] and index [`gfx::Buffer`], to simplify managing and
/// rendering common indexed geometry.
pub struct Mesh<V: gfx::Vertex> {
	pub vao: gfx::Vao,
	pub vertex_buffer: gfx::Buffer<V>,
	pub index_buffer: gfx::Buffer<u16>,
}


impl<V: gfx::Vertex> Mesh<V> {
	pub fn with_buffer_usage(gfx: &mut gfx::ResourceContext<'_>, buffer_usage: gfx::BufferUsage) -> Self {
		let mut vao = gfx.new_vao();

		let vertex_buffer = gfx.new_buffer(buffer_usage);
		let index_buffer = gfx.new_buffer(buffer_usage);

		vao.bind_vertex_buffer(0, vertex_buffer);
		vao.bind_index_buffer(index_buffer);

		Mesh {
			vao,
			vertex_buffer,
			index_buffer,
		}
	}

	pub fn new(gfx: &mut gfx::ResourceContext<'_>) -> Self {
		Mesh::with_buffer_usage(gfx, gfx::BufferUsage::Stream)
	}

	pub fn from_mesh_data(gfx: &mut gfx::ResourceContext<'_>, mesh_data: &MeshData<V>) -> Self {
		let mut mesh = Mesh::with_buffer_usage(gfx, gfx::BufferUsage::Static);
		mesh.upload(mesh_data);
		mesh
	}

	pub fn draw(&self, gfx: &mut gfx::DrawContext<'_>, draw_mode: gfx::DrawMode) {
		gfx.bind_vao(self.vao);
		gfx.draw_indexed(draw_mode, self.index_buffer.len());
	}

	pub fn draw_instanced(&self, gfx: &mut gfx::DrawContext<'_>, draw_mode: gfx::DrawMode, num_instances: u32) {
		gfx.bind_vao(self.vao);
		gfx.draw_instances_indexed(draw_mode, self.index_buffer.len(), num_instances);
	}

	pub fn upload(&mut self, mesh_data: &MeshData<V>) {
		self.vertex_buffer.upload(&mesh_data.vertices);
		self.index_buffer.upload(&mesh_data.indices);
	}

	pub fn upload_separate(&mut self, vertices: &[V], indices: &[u16]) {
		self.vertex_buffer.upload(vertices);
		self.index_buffer.upload(indices);
	}
}



/// Aggregates a [`gfx::Vao`] and vertex [`gfx::Buffer`], to simplify managing and
/// rendering common non-indexed geometry.
pub struct BasicMesh<V: gfx::Vertex> {
	pub vao: gfx::Vao,
	pub vertex_buffer: gfx::Buffer<V>,
}


impl<V: gfx::Vertex> BasicMesh<V> {
	pub fn with_buffer_usage(gfx: &mut gfx::ResourceContext<'_>, buffer_usage: gfx::BufferUsage) -> Self {
		let mut vao = gfx.new_vao();

		let vertex_buffer = gfx.new_buffer(buffer_usage);
		vao.bind_vertex_buffer(0, vertex_buffer);

		BasicMesh {
			vao,
			vertex_buffer,
		}
	}

	pub fn new(gfx: &mut gfx::ResourceContext<'_>) -> Self {
		BasicMesh::with_buffer_usage(gfx, gfx::BufferUsage::Stream)
	}

	pub fn from_vertices(gfx: &mut gfx::ResourceContext<'_>, vertices: &[V]) -> Self {
		let mut mesh = BasicMesh::with_buffer_usage(gfx, gfx::BufferUsage::Static);
		mesh.upload(vertices);
		mesh
	}

	pub fn draw(&self, gfx: &mut gfx::DrawContext<'_>, draw_mode: gfx::DrawMode) {
		gfx.bind_vao(self.vao);
		gfx.draw_arrays(draw_mode, self.vertex_buffer.len());
	}

	pub fn upload(&mut self, vertices: &[V]) {
		self.vertex_buffer.upload(vertices);
	}
}



/// Geometry data to be uploaded to a [`Mesh`].
/// See [`ColorMeshBuilder`] for a common usecase for [`MeshData`].
pub struct MeshData<V: gfx::Vertex> {
	pub vertices: Vec<V>,
	pub indices: Vec<u16>,
}


impl<V: gfx::Vertex> MeshData<V> {
	pub fn new() -> Self {
		MeshData {
			vertices: Vec::new(),
			indices: Vec::new(),
		}
	}

	pub fn clear(&mut self) {
		self.vertices.clear();
		self.indices.clear();
	}

	pub fn extend(&mut self, vs: impl IntoIterator<Item=V>, is: impl IntoIterator<Item=u16>) {
		let index_start = self.vertices.len() as u16;
		self.vertices.extend(vs);
		self.indices.extend(is.into_iter().map(|idx| index_start + idx));
	}
}
