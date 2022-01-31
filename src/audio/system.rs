use crate::prelude::*;
use crate::audio::nodes::Node;
use crate::audio::node_graph::{NodeGraph, NodeId};
use crate::audio::ringbuffer::{Ringbuffer, RingbufferReadLock};

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};


pub struct EvaluationContext<'sys> {
	pub sample_rate: f32,
	pub resources: &'sys Resources,
}



pub struct AudioSystem {
	audio_device: sdl2::audio::AudioDevice<AudioSubmissionWorker>,
	inner: Arc<Mutex<Inner>>,

	running: Arc<AtomicBool>,
	producer_thread_handle: Option<JoinHandle<()>>,
}


impl AudioSystem {
	pub fn new(sdl_audio: sdl2::AudioSubsystem) -> Result<AudioSystem, Box<dyn Error>> {
		let desired_spec = sdl2::audio::AudioSpecDesired {
			freq: Some(44100),
			channels: Some(2),
			samples: Some(128),
		};

		let inner = Inner {
			node_graph: NodeGraph::new(),
			resources: Resources::new(),
			sample_rate: 44100.0,
		};

		let inner = Arc::new(Mutex::new(inner));
		let sample_buffer = Arc::new(Ringbuffer::new(128*10));
		let running = Arc::new(AtomicBool::new(true));

		// TODO(pat.m): if any of the below functions fail, then its possible for this thread never to be killed.
		// make sure its killed properly on failure
		let producer_thread_builder = thread::Builder::new().name("audio producer".into());
		let producer_thread_handle = producer_thread_builder.spawn({
			let inner = inner.clone();
			let running = running.clone();
			let sample_buffer = sample_buffer.clone();
 
			move || {
				audio_producer_worker(inner, sample_buffer, running)
			}
		})?;

		let create_submission_worker = |spec: sdl2::audio::AudioSpec| {
			assert!(spec.freq == 44100);
			assert!(spec.channels == 2);
			{
				let mut inner_mut = inner.lock().unwrap();
				inner_mut.sample_rate = spec.freq as f32;
			}

			AudioSubmissionWorker {
				sample_buffer: sample_buffer.clone(),
			}
		};

		let audio_device = sdl_audio.open_playback(None, &desired_spec, create_submission_worker)?;
		audio_device.resume();

		Ok(AudioSystem {
			audio_device,
			inner,

			running,
			producer_thread_handle: Some(producer_thread_handle),
		})
	}


	pub fn update(&mut self) {
		// Doesn't have to happen that often really
		let mut inner_lock = self.inner.lock().unwrap();
		let inner = &mut *inner_lock;

		let sample_rate = inner.sample_rate;

		let eval_ctx = EvaluationContext {
			sample_rate,
			resources: &inner.resources,
		};

		inner.node_graph.cleanup_finished_nodes(&eval_ctx);
	}

	pub fn output_node(&self) -> NodeId {
		self.inner.lock().unwrap().node_graph.output_node()
	}

	pub fn update_graph<F, R>(&mut self, f: F) -> R
		where F: FnOnce(&mut NodeGraph) -> R
	{
		let mut inner = self.inner.lock().unwrap();
		f(&mut inner.node_graph)
	}

	pub fn add_node(&mut self, node: impl Node) -> NodeId {
		self.update_graph(move |graph| graph.add_node(node, false))
	}

	pub fn add_ephemeral_node(&mut self, node: impl Node) -> NodeId {
		self.update_graph(move |graph| graph.add_node(node, true))
	}

	pub fn add_send(&mut self, node: NodeId, target: NodeId) {
		self.update_graph(move |graph| graph.add_send(node, target))
	}

	pub fn add_node_with_send(&mut self, node: impl Node, send_node: NodeId) -> NodeId {
		self.update_graph(move |graph| {
			let node_id = graph.add_node(node, false);
			graph.add_send(node_id, send_node);
			node_id
		})
	}

	pub fn remove_node(&mut self, node: NodeId) {
		self.update_graph(move |graph| graph.remove_node(node))
	}

	pub fn add_sound(&mut self, buffer: Vec<f32>) -> SoundId {
		let key = self.inner.lock().unwrap().resources.buffers.insert(buffer);
		SoundId(key)
	}

	
	// pub fn add_parameter<T: ParameterData>(&mut self, initial_value: T) -> ParameterId<T> {
	// 	todo!()
	// }

	// pub fn push_parameter<T: ParameterData>(&mut self, param: ParameterId<T>, value: T) {
	// 	todo!()
	// }
}


impl Drop for AudioSystem {
	fn drop(&mut self) {
		self.running.store(false, Ordering::Relaxed);

		if let Some(handle) = self.producer_thread_handle.take() {
			handle.join().unwrap();
		}
	}
}




#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SoundId(ResourceKey);

slotmap::new_key_type! {
	pub(in crate::audio) struct ResourceKey;
	pub(in crate::audio) struct ParameterKey;
}

pub struct Resources {
	buffers: slotmap::SlotMap<ResourceKey, Vec<f32>>,
}

impl Resources {
	fn new() -> Resources {
		Resources {
			buffers: slotmap::SlotMap::with_key(),
			// f32_parameters: slotmap::SlotMap::with_key(),
		}
	}

	pub fn get(&self, sound_id: SoundId) -> &[f32] {
		&self.buffers[sound_id.0]
	}
}





struct Inner {
	node_graph: NodeGraph,
	resources: Resources,
	sample_rate: f32,
}




struct AudioSubmissionWorker {
	sample_buffer: Arc<Ringbuffer<f32>>,
}

impl sdl2::audio::AudioCallback for AudioSubmissionWorker {
	type Channel = f32;

	fn callback(&mut self, output: &mut [Self::Channel]) {
		let lock @ RingbufferReadLock {presplit, postsplit, ..} = self.sample_buffer.lock_for_read(output.len());
		let total_len = lock.len();

		output[..presplit.len()].copy_from_slice(presplit);
		output[presplit.len()..total_len].copy_from_slice(postsplit);

		if total_len < output.len() {
			// Buffer underflow - fill with zeroes
			output[total_len..].fill(0.0);
		}
	}
}



fn audio_producer_worker(inner: Arc<Mutex<Inner>>, sample_buffer: Arc<Ringbuffer<f32>>, running: Arc<AtomicBool>) {
	set_realtime_thread_priority();

	while running.load(Ordering::Relaxed) {
		let mut inner = inner.lock().unwrap();
		let Inner {node_graph, resources, sample_rate} = &mut *inner;

		node_graph.update_topology();

		// TODO(pat.m): get this number from the node graph
		let buffer_size = 2*256;

		loop {
			if sample_buffer.free_capacity() < buffer_size {
				break
			}

			let eval_ctx = EvaluationContext {
				sample_rate: *sample_rate,
				resources,
			};

			let buffer = node_graph.process(&eval_ctx);

			let write_lock = sample_buffer.lock_for_write(buffer.len());
			assert!(write_lock.len() == buffer.len());

			let split_len = write_lock.presplit.len();
			write_lock.presplit.copy_from_slice(&buffer[..split_len]);
			write_lock.postsplit.copy_from_slice(&buffer[split_len..]);
		}

		drop(inner);

		thread::sleep(std::time::Duration::from_millis(2));
	}
}



fn set_realtime_thread_priority() {
	#[cfg(windows)] {
		use winapi::um::processthreadsapi::{GetCurrentThread, SetThreadPriority};

		let priority = winapi::um::winbase::THREAD_PRIORITY_TIME_CRITICAL;

		let set_priority_succeeded = unsafe {
			let current_thread = GetCurrentThread();
			SetThreadPriority(current_thread, priority as i32) != 0
		};

		assert!(set_priority_succeeded);

		// TODO(pat.m): GetLastError FormatMessage
	}
}