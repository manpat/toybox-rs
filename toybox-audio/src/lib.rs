#![feature(let_chains)]

use cpal::traits::*;

use anyhow::Context as AnyhowContext;
use tracing::instrument;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle};


pub mod prelude {
	pub use super::Provider;
}


#[instrument(skip_all, name="audio init")]
pub fn init() -> System {
	if false {
		std::thread::spawn(enumerate_audio_devices);
	}

	let shared = Arc::new(SharedState {
		provider: Mutex::new(None),
		device_lost: AtomicBool::new(false),
	});

	System {
		stream_state: StreamState::Pending(Some(start_stream_build(shared.clone()))),
		shared,
	}
}

#[instrument]
fn enumerate_audio_devices() -> anyhow::Result<()> {
	let host = cpal::default_host();

	log::trace!("vvvvvvv Available audio devices vvvvvvvv");

	for device in host.output_devices()? {
		log::trace!("   => {}", device.name()?);
		log::trace!("      => default output config: {:?}", device.default_output_config()?);
		log::trace!("      => supported configs:");
		for config in device.supported_output_configs()? {
			log::trace!("         => {config:?}");
		}
		log::trace!("");
	}

	log::trace!("^^^^^^ Available audio devices ^^^^^^^");

	Ok(())
}


pub struct System {
	shared: Arc<SharedState>,

	stream_state: StreamState,
}

impl System {
	pub fn update(&mut self) {
		match &mut self.stream_state {
			StreamState::Active(_) => {
				if self.shared.device_lost.load(Ordering::Relaxed) {
					self.stream_state = StreamState::Pending(Some(start_stream_build(self.shared.clone())));
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
						self.shared.device_lost.store(false, Ordering::Relaxed);
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
		let mut shared_provider = self.shared.provider.lock().unwrap();

		if let Some(mut provider) = provider.into() {
			let configuration = self.stream_state.as_active_stream().map(|active_stream| active_stream.configuration);

			log::info!("Setting initial provider configuration: {configuration:?}");
			provider.on_configuration_changed(configuration);

			*shared_provider = Some(Box::new(provider));

		} else {
			*shared_provider = None;
		}

		Ok(())
	}

	fn try_update_provider_config(&mut self) {
		if let Ok(mut guard) = self.shared.provider.lock()
			&& let Some(provider) = &mut *guard
		{
			let configuration = self.stream_state.as_active_stream()
				.map(|active_stream| active_stream.configuration);

			log::info!("Update provider configuration: {configuration:?}");
			provider.on_configuration_changed(configuration);
		}
	}
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


struct SharedState {
	provider: Mutex<Option<Box<dyn Provider>>>,
	device_lost: AtomicBool,
}


// should be able to close and reopen streams dynamically, potentially on different devices
//	any non-device state should be maintained
// 	should be able to cope with different sample rates


fn start_stream_build(shared: Arc<SharedState>) -> JoinHandle<anyhow::Result<ActiveStream>> {
	std::thread::spawn(move || {
		let host = cpal::default_host();
		build_output_stream(&host, shared.clone())
	})
}


#[instrument(skip_all, name="audio build_output_stream")]
fn build_output_stream(host: &cpal::Host, shared: Arc<SharedState>) -> anyhow::Result<ActiveStream> {
	let device = host.default_output_device().context("no output device available")?;

	log::info!("Selected audio device: {}", device.name().unwrap_or_else(|_| String::from("<no name>")));

	let supported_configs_range = device.supported_output_configs()
		.context("error while querying configs")?;

	// TODO(pat.m): support different sample formats
	let supported_config = supported_configs_range
		.filter(|config| config.sample_format().is_float())
		.max_by(cpal::SupportedStreamConfigRange::cmp_default_heuristics)
		.context("couldn't find a supported configuration")?;

	let desired_sample_rate = 48000.clamp(supported_config.min_sample_rate().0, supported_config.max_sample_rate().0);
	let supported_config = supported_config
		.with_sample_rate(cpal::SampleRate(desired_sample_rate));

	let config = supported_config.into();

	log::info!("Selected audio device config: {config:#?}");

	let stream = device.build_output_stream(
		&config,
		{
			let shared = Arc::clone(&shared);

			move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
				let _span = tracing::trace_span!("audio provider callback").entered();

				let mut provider_maybe = shared.provider.lock().unwrap();
				if let Some(provider) = &mut *provider_maybe {
					provider.fill_buffer(data);
				} else {
					data.fill(0.0);
				}
			}
		},
		{
			move |err| {
				// react to errors here.
				log::warn!("audio device lost! {err}");
				shared.device_lost.store(true, Ordering::Relaxed);
			}
		},
		None // None=blocking, Some(Duration)=timeout
	)?;

	stream.play()?;

	let configuration = Configuration {
		sample_rate: config.sample_rate.0 as u32,
		channels: config.channels as usize,
	};

	Ok(ActiveStream {
		// We pass around the StreamInner so that we can avoid the !Sync/!Send bounds on Stream.
		// Send/Sync are disabled because of android, but we don't care about that.
		// https://docs.rs/cpal/latest/x86_64-pc-windows-msvc/src/cpal/platform/mod.rs.html#67
		_stream: stream.into_inner(),
		configuration
	})
}

struct ActiveStream {
	_stream: cpal::platform::StreamInner,
	configuration: Configuration,
}

enum StreamState {
	Pending(Option<JoinHandle<anyhow::Result<ActiveStream>>>),
	Active(ActiveStream),
	InitFailure,
}

impl StreamState {
	fn as_active_stream(&self) -> Option<&ActiveStream> {
		match self {
			StreamState::Active(active_stream) => Some(active_stream),
			_ => None
		}
	}
}