use crate::gfx::{self, raw};

#[derive(Copy, Clone, Debug)]
pub struct Vao {
	pub(super) handle: u32,
}



impl Vao {
	pub(super) fn new(handle: u32) -> Vao {
		Vao {
			handle,
		}
	}


	pub fn bind_index_buffer(&mut self, index_buffer: gfx::Buffer<u16>) {
		unsafe {
			raw::VertexArrayElementBuffer(self.handle, index_buffer.handle);
		}
	}

	pub fn bind_vertex_buffer<V: gfx::Vertex>(&mut self, binding: u32, vertex_buffer: gfx::Buffer<V>) {
		use gfx::vertex::AttributeTypeFormat;

		let descriptor = V::descriptor();
		let stride = descriptor.size_bytes as i32;

		for (attribute_index, attribute) in descriptor.attributes.iter().enumerate() {
			let attribute_index = attribute_index as u32;

			let &gfx::vertex::Attribute{
				offset_bytes,
				num_elements,
				gl_type,
				format,
			} = attribute;

			unsafe {
				raw::EnableVertexArrayAttrib(self.handle, attribute_index);
				raw::VertexArrayAttribBinding(self.handle, attribute_index, binding);

				match format {
					AttributeTypeFormat::Float =>
						raw::VertexArrayAttribFormat(self.handle, attribute_index, num_elements as i32, gl_type, false as u8, offset_bytes),

					AttributeTypeFormat::NormalisedInt =>
						raw::VertexArrayAttribFormat(self.handle, attribute_index, num_elements as i32, gl_type, true as u8, offset_bytes),

					AttributeTypeFormat::Integer =>
						raw::VertexArrayAttribIFormat(self.handle, attribute_index, num_elements as i32, gl_type, offset_bytes),
				};
			}
		}

		unsafe {
			raw::VertexArrayVertexBuffer(self.handle, binding, vertex_buffer.handle, 0, stride);
		}
	}
}
