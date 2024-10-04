#![feature(let_chains)]

use cpal::traits::*;

use anyhow::Context as AnyhowContext;
use tracing::instrument;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};


pub mod prelude {
	pub use super::Provider;
}


#[instrument(skip_all, name="audio init")]
pub fn init() -> anyhow::Result<System> {
	let host = cpal::default_host();

	if false {
		let _span = tracing::info_span!("enumerate audio devices").entered();

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
	}

	let shared = Arc::new(SharedState {
		provider: Mutex::new(None),
		device_lost: AtomicBool::new(false),
	});

	match build_output_stream(&host, &shared) {
		Ok(active_stream) => {
			Ok(System {
				active_stream: Some(active_stream),
				host,
				shared,
			})
		}

		Err(error) => {
			log::error!("Failed to create audio device: {error}");

			// Prevent system from trying again.
			// TODO(pat.m): try on a time out? listen for audio device changes?
			// TODO(pat.m): this definitely needs to be renamed
			shared.device_lost.store(true, Ordering::Relaxed);

			Ok(System {
				active_stream: None,
				host,
				shared,
			})
		}
	}

}

pub struct System {
	host: cpal::Host,
	shared: Arc<SharedState>,

	active_stream: Option<ActiveStream>,
}

impl System {
	pub fn update(&mut self) {
		if self.shared.device_lost.load(Ordering::Relaxed) {
			if self.active_stream.is_some() {
				self.active_stream = None;
			} else {
				// If device_lost and there's no existing stream, then we've given up on creating a device for now
				return;
			}
		}

		if self.active_stream.is_some() {
			return;
		}

		// Try and build a new stream
		match build_output_stream(&self.host, &self.shared) {
			Ok(active_stream) => {
				if let Ok(mut guard) = self.shared.provider.lock()
					&& let Some(provider) = &mut *guard
				{
					provider.on_configuration_changed(Some(active_stream.configuration));
				}

				self.active_stream = Some(active_stream);
				self.shared.device_lost.store(false, Ordering::Relaxed);
			}

			Err(error) => {
				log::error!("Failed to recreate audio device: {error}");

				if let Ok(mut guard) = self.shared.provider.lock()
					&& let Some(provider) = &mut *guard
				{
					provider.on_configuration_changed(None);
				}
			}
		};
	}
}

impl System {
	pub fn set_provider<P>(&mut self, provider: impl Into<Option<P>>) -> anyhow::Result<()>
		where P : Provider + Send
	{
		let mut shared_provider = self.shared.provider.lock().unwrap();

		if let Some(mut provider) = provider.into() {
			let configuration = self.active_stream.as_ref().map(|active_stream| active_stream.configuration);
			provider.on_configuration_changed(configuration);

			*shared_provider = Some(Box::new(provider));

		} else {
			*shared_provider = None;
		}

		Ok(())
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


#[instrument(skip_all, name="audio build_output_stream")]
fn build_output_stream(host: &cpal::Host, shared: &Arc<SharedState>) -> anyhow::Result<ActiveStream> {
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
			let shared = Arc::clone(&shared);

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

	Ok(ActiveStream{_stream: stream, configuration})
}

struct ActiveStream {
	_stream: cpal::Stream,
	configuration: Configuration,
}