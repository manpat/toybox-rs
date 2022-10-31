use crate::prelude::*;
use crate::audio::nodes::Node;
use crate::audio::node_graph::{NodeGraph, NodeId};
use crate::audio::ringbuffer::{Ringbuffer, RingbufferReadLock};

use crate::utility::{ResourceScopeID, ResourceScopeToken};

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};


pub struct EvaluationContext<'sys> {
	pub sample_rate: f32,
	pub resources: &'sys Resources,
}

enum ProducerCommand {
	UpdateGraph(Box<dyn FnOnce(&mut NodeGraph) + Send + 'static>),
	RemoveNodesPinnedToScope(ResourceScopeID),
}


pub struct AudioSystem {
	_audio_device: sdl2::audio::AudioDevice<AudioSubmissionWorker>,
	shared: Arc<Shared>,
	command_tx: Sender<ProducerCommand>,

	// expired_resource_scopes: Vec<ResourceScopeID>,

	// Option so we can take ownership of it in Drop
	producer_thread_handle: Option<JoinHandle<()>>,
}


// Public API
impl AudioSystem {
	pub fn output_node(&self) -> NodeId {
		self.shared.inner.lock().unwrap().node_graph.output_node()
	}

	/// Queues callback `f` to be run on the audio producer thread given the NodeGraph.
	/// Use for updates that don't require feedback.
	pub fn queue_update<F>(&mut self, f: F)
		where F: FnOnce(&mut NodeGraph) + Send + 'static
	{
		let f_boxed = Box::new(f);
		self.command_tx
			.send(ProducerCommand::UpdateGraph(f_boxed))
			.unwrap();
	}

	/// Runs callback `f` with the NodeGraph and returns its result.
	/// Locks the shared state for the duration of the call, so prefer `queue_update` when the result isn't required.
	pub fn update_graph_immediate<F, R>(&mut self, f: F) -> R
		where F: FnOnce(&mut NodeGraph) -> R
	{
		let mut inner = self.shared.inner.lock().unwrap();
		f(&mut inner.node_graph)
	}

	pub fn add_node(&mut self, node: impl Node) -> NodeId {
		self.update_graph_immediate(move |graph| graph.add_node(node, None))
	}

	pub fn add_send(&mut self, node: NodeId, target: NodeId) {
		self.queue_update(move |graph| graph.add_send(node, target))
	}

	pub fn add_node_with_send(&mut self, node: impl Node, send_node: NodeId) -> NodeId {
		self.update_graph_immediate(move |graph| graph.add_node(node, send_node))
	}

	pub fn remove_node(&mut self, node: NodeId) {
		self.queue_update(move |graph| graph.remove_node(node))
	}

	pub fn add_sound(&mut self, buffer: Vec<f32>) -> SoundId {
		// TODO(pat.m): resource scopes
		let key = self.shared.inner.lock().unwrap().resources.buffers.insert(buffer);
		SoundId(key)
	}
}


// Private API
impl AudioSystem {
	pub(crate) fn new(sdl_audio: sdl2::AudioSubsystem, global_scope_token: ResourceScopeToken) -> Result<AudioSystem, Box<dyn Error>> {
		// Set realtime priority for rayon worker threads - since they are mainly used for audio.
		// TODO(pat.m): separate thread pool for audio workers?
		rayon::ThreadPoolBuilder::new()
			.start_handler(|_| set_realtime_thread_priority())
			.build_global()?;

		let sample_rate = 44100;
		let requested_frame_samples = 128;
		let requested_buffer_size = 2 * requested_frame_samples;

		let desired_spec = sdl2::audio::AudioSpecDesired {
			freq: Some(sample_rate),
			channels: Some(2),
			samples: Some(requested_frame_samples as u16),
		};

		let inner = Inner {
			node_graph: NodeGraph::new(global_scope_token.id()),
			resources: Resources::new(),
		};

		// TODO(pat.m): figure out how to tune this
		let requested_ringbuffer_size = (6 * sample_rate as usize) / 60;

		let shared = Arc::new(Shared {
			inner: Mutex::new(inner),
			sample_buffer: Ringbuffer::new(requested_ringbuffer_size),
			running: AtomicBool::new(true),
			producer_cond_var: AtomicBool::new(true),
			producer_thread: Mutex::new(None),
			sample_rate: sample_rate as f32,
		});

		{
			let buff_size = shared.sample_buffer.capacity();
			let samples = buff_size / 2;
			let millis = (samples * 1000) as f64 / sample_rate as f64;
			println!("ringbuffer size: {millis:2.2}ms");
		}

		assert!(shared.sample_buffer.capacity() >= requested_buffer_size, "Sample ringbuffer not large enough for requested audio frame size");

		let (command_tx, command_rx) = mpsc::channel();

		// TODO(pat.m): if any of the below functions fail, then its possible for this thread never to be killed.
		// make sure its killed properly on failure
		let producer_thread_builder = thread::Builder::new().name("audio producer".into());
		let producer_thread_handle = producer_thread_builder.spawn({
			let shared = shared.clone();
 
			move || {
				audio_producer_worker(shared, command_rx)
			}
		})?;

		*shared.producer_thread.lock().unwrap() = Some(producer_thread_handle.thread().clone());

		let create_submission_worker = |spec: sdl2::audio::AudioSpec| {
			assert!(spec.freq == sample_rate);
			assert!(spec.channels == 2);

			println!("audio spec buffer size {} -> {}", spec.size, requested_ringbuffer_size);

			AudioSubmissionWorker {
				shared: shared.clone(),
			}
		};

		let audio_device = sdl_audio.open_playback(None, &desired_spec, create_submission_worker)?;
		audio_device.resume();

		Ok(AudioSystem {
			_audio_device: audio_device,
			shared,
			command_tx,

			producer_thread_handle: Some(producer_thread_handle),
		})
	}

	pub(crate) fn cleanup_resource_scope(&mut self, scope_id: ResourceScopeID) {
		self.command_tx
			.send(ProducerCommand::RemoveNodesPinnedToScope(scope_id))
			.unwrap();
	}
}


impl Drop for AudioSystem {
	fn drop(&mut self) {
		self.shared.running.store(false, Ordering::Relaxed);

		if let Some(handle) = self.producer_thread_handle.take() {
			handle.join().unwrap();
		}
	}
}




#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SoundId(ResourceKey);

slotmap::new_key_type! {
	pub(in crate::audio) struct ResourceKey;
}

pub struct Resources {
	buffers: slotmap::SlotMap<ResourceKey, Vec<f32>>,
}

impl Resources {
	fn new() -> Resources {
		Resources {
			buffers: slotmap::SlotMap::with_key(),
		}
	}

	pub fn get(&self, sound_id: SoundId) -> &[f32] {
		&self.buffers[sound_id.0]
	}
}





struct Inner {
	node_graph: NodeGraph,
	resources: Resources,
}

/// State that is shared between audio worker threads and main thread.
struct Shared {
	inner: Mutex<Inner>,

	/// The Ringbuffer used to push ready samples from audio producer thread to audio submission thread.
	/// It should not be accessed by anything other than these two threads after startup!
	sample_buffer: Ringbuffer<f32>,

	/// Stores whether the audio system is currently running. Set to false on shutdown so producer thread can
	/// shut down gracefully.
	running: AtomicBool,

	producer_cond_var: AtomicBool,
	producer_thread: Mutex<Option<thread::Thread>>,

	/// The audio sample rate. Typically 44100Hz or 48000Hz.
	sample_rate: f32,
}


/// The `AudioCallback` responsible for submitting ready samples from the ringbuffer to the audio device.
/// Owned by the sdl audio device and invoked from an audio thread.
struct AudioSubmissionWorker {
	shared: Arc<Shared>,
}

impl sdl2::audio::AudioCallback for AudioSubmissionWorker {
	type Channel = f32;

	#[instrument(skip_all, name = "AudioSubmissionWorker::callback", fields(samples=output.len()))]
	fn callback(&mut self, mut output: &mut [Self::Channel]) {
		loop {
			let lock @ RingbufferReadLock {presplit, postsplit, ..} = self.shared.sample_buffer.lock_for_read(output.len());
			let total_len = lock.len();

			output[..presplit.len()].copy_from_slice(presplit);
			output[presplit.len()..total_len].copy_from_slice(postsplit);

			// The buffer has been filled completely, so we can finish
			if total_len >= output.len() {
				assert!(total_len == output.len());
				break
			}

			// Otherwise not enough samples were ready in time, so we will wait a little bit and try again.
			tracing::info!("audio underrun! {}", output.len() - total_len);
			output = &mut output[total_len..];

			let mut spin_count = 1000;
			while self.shared.sample_buffer.available_samples() < output.len() && spin_count > 0 {
				std::hint::spin_loop();
				spin_count -= 1;
			}

			// If we spin for too long, clear the rest of the buffer and bail
			if spin_count <= 0 {
				tracing::info!("audio timeout!");
				output.fill(0.0);
				break;
			}
		}

		self.shared.producer_cond_var.store(true, Ordering::Relaxed);
		self.shared.producer_thread.lock().unwrap().as_ref().unwrap().unpark();
	}
}



/// The audio producer worker thread body. Responsible for generating samples to be consumed by `AudioSubmissionWorker`.
#[instrument(skip_all)]
fn audio_producer_worker(shared: Arc<Shared>, command_rx: Receiver<ProducerCommand>) {
	set_realtime_thread_priority();

	let sample_rate = shared.sample_rate;

	let mut expired_resource_scopes = Vec::new();

	while shared.running.load(Ordering::Relaxed) {
		shared.producer_cond_var.store(false, Ordering::Relaxed);

		let mut inner = shared.inner.lock().unwrap();
		let Inner {ref mut node_graph, ref resources, ..} = *inner;

		for cmd in command_rx.try_iter() {
			match cmd {
				ProducerCommand::UpdateGraph(func) => {
					func(node_graph);
				}

				ProducerCommand::RemoveNodesPinnedToScope(scope_id) => {
					expired_resource_scopes.push(scope_id);
				}
			}
		}

		expired_resource_scopes.sort();

		let eval_ctx = EvaluationContext {sample_rate, resources};
		node_graph.cleanup_finished_nodes(&eval_ctx, &expired_resource_scopes);
		node_graph.update_topology(&eval_ctx);

		expired_resource_scopes.clear();

		let stereo_buffer_size = 2 * node_graph.buffer_size();

		// Hard limit loop count to avoid deadlocks when consumer thread outpaces producer thread.
		let mut loop_count = 5;

		while loop_count > 0 {
			if shared.sample_buffer.free_capacity() < stereo_buffer_size {
				break
			}

			let eval_ctx = EvaluationContext {sample_rate, resources};
			let buffer = node_graph.process(&eval_ctx);

			let write_lock = shared.sample_buffer.lock_for_write(buffer.len());
			assert!(write_lock.len() == buffer.len());

			let split_len = write_lock.presplit.len();
			write_lock.presplit.copy_from_slice(&buffer[..split_len]);
			write_lock.postsplit.copy_from_slice(&buffer[split_len..]);

			loop_count -= 1;
		}

		// Holding locks across sleeps is bad
		drop(inner);

		if loop_count <= 0 {
			tracing::info!("audio producer thread took too long!");
		}

		while !shared.producer_cond_var.load(Ordering::Relaxed) && shared.running.load(Ordering::Relaxed) {
			thread::park();
		}
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

	#[cfg(linux)] unsafe {
		let mut sched_param = libc::sched_param { sched_priority: 80 };
		libc::sched_setscheduler(0, libc::SCHED_FIFO, &sched_param);
	}
}