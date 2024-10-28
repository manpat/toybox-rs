#![feature(let_chains)]

use cpal::traits::*;

use anyhow::Context as AnyhowContext;
use tracing::instrument;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle};

mod device;
use device::*;

pub mod prelude {
	pub use super::Provider;
}


#[derive(Debug, Copy, Clone)]
pub struct Configuration {
	pub sample_rate: u32,
	pub channels: usize,
}

pub trait Provider : Send + 'static {
	fn on_configuration_changed(&mut self, _: Option<Configuration>);
	fn fill_buffer(&mut self, buffer: &mut [f32]);
}


pub struct System {
	stream_shared: Arc<SharedStreamState>,
	stream_state: StreamState,
}

impl System {
	#[instrument(skip_all, name="audio init")]
	pub fn init() -> System {
		if false {
			std::thread::spawn(enumerate_audio_devices);
		}

		let stream_shared = Arc::new(SharedStreamState {
			provider: Mutex::new(None),
			device_lost: AtomicBool::new(false),
		});

		System {
			stream_state: StreamState::Pending(Some(start_stream_build(stream_shared.clone()))),
			stream_shared,
		}
	}

	pub fn update(&mut self) {
		match &mut self.stream_state {
			StreamState::Active(_) => {
				if self.stream_shared.device_lost.load(Ordering::Relaxed) {
					self.stream_state = StreamState::Pending(Some(start_stream_build(self.stream_shared.clone())));
				}
			}

			StreamState::Pending(handle) => {
				if !handle.as_ref().unwrap().is_finished() {
					return;
				}

				match handle.take().unwrap().join() {
					Ok(Ok(new_stream)) => {
						log::info!("Output stream active");

						self.stream_state = StreamState::Active(new_stream);
						self.stream_shared.device_lost.store(false, Ordering::Relaxed);
					}

					Ok(Err(error)) => {
						log::error!("Failed to build audio stream: {error}");
						self.stream_state = StreamState::InitFailure;
					}

					Err(panic_data) => {
						log::error!("Panic during audio stream creation!");
						self.stream_state = StreamState::InitFailure;
						self.try_update_provider_config();

						std::panic::resume_unwind(panic_data);
					}
				}

				self.try_update_provider_config();
			}

			StreamState::InitFailure => {}
		}
	}
}

impl System {
	pub fn set_provider<P>(&mut self, provider: impl Into<Option<P>>) -> anyhow::Result<()>
		where P : Provider + Send
	{
		let mut shared_provider = self.stream_shared.provider.lock().unwrap();

		if let Some(mut provider) = provider.into() {
			let configuration = self.stream_state.current_configuration();

			log::info!("Setting initial provider configuration: {configuration:?}");
			provider.on_configuration_changed(configuration);

			*shared_provider = Some(Box::new(provider));

		} else {
			*shared_provider = None;
		}

		Ok(())
	}

	fn try_update_provider_config(&mut self) {
		let configuration = self.stream_state.current_configuration();

		if let Ok(mut guard) = self.stream_shared.provider.lock()
			&& let Some(provider) = &mut *guard
		{
			log::info!("Update provider configuration: {configuration:?}");
			provider.on_configuration_changed(configuration);
		}
	}
}
