use crate::gfx::raw;

#[derive(Copy, Clone, Debug)]
pub struct Capabilities {
	pub max_simultaneous_time_elapsed_queries: i32,
	pub max_simultaneous_primitive_queries: i32,

	pub ubo_offset_alignment: i32,
	pub ssbo_offset_alignment: i32,
}


impl Capabilities {
	pub(super) fn new() -> Capabilities {
		let mut max_simultaneous_time_elapsed_queries = 0;
		let mut max_simultaneous_primitive_queries = 0;

		let mut ubo_offset_alignment = 0;
		let mut ssbo_offset_alignment = 0;

		unsafe {
			raw::GetQueryiv(raw::TIME_ELAPSED, raw::QUERY_COUNTER_BITS, &mut max_simultaneous_time_elapsed_queries);
			raw::GetQueryiv(raw::PRIMITIVES_GENERATED, raw::QUERY_COUNTER_BITS, &mut max_simultaneous_primitive_queries);

			raw::GetIntegerv(raw::UNIFORM_BUFFER_OFFSET_ALIGNMENT, &mut ubo_offset_alignment);
			raw::GetIntegerv(raw::SHADER_STORAGE_BUFFER_OFFSET_ALIGNMENT, &mut ssbo_offset_alignment);
		}

		Capabilities {
			max_simultaneous_time_elapsed_queries,
			max_simultaneous_primitive_queries,
			ubo_offset_alignment,
			ssbo_offset_alignment,
		}
	}
}