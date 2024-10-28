use cpal::traits::*;
use anyhow::Context as AnyhowContext;
use tracing::instrument;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle};

use super::{Configuration, Provider};


// should be able to close and reopen streams dynamically, potentially on different devices
//	any non-device state should be maintained
// 	should be able to cope with different sample rates

pub struct SharedStreamState {
	pub provider: Mutex<Option<Box<dyn Provider>>>,
	pub device_lost: AtomicBool,
}




pub struct ActiveStream {
	_stream: cpal::platform::StreamInner,
	configuration: Configuration,
}

pub enum StreamState {
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

	pub fn current_configuration(&self) -> Option<Configuration> {
		self.as_active_stream()
			.map(|active_stream| active_stream.configuration)
	}
}


pub fn start_stream_build(stream_shared: Arc<SharedStreamState>) -> JoinHandle<anyhow::Result<ActiveStream>> {
	std::thread::spawn(move || {
		let host = cpal::default_host();
		build_output_stream(&host, stream_shared.clone())
	})
}


#[instrument(skip_all, name="audio build_output_stream")]
fn build_output_stream(host: &cpal::Host, stream_shared: Arc<SharedStreamState>) -> anyhow::Result<ActiveStream> {
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
			let stream_shared = Arc::clone(&stream_shared);

			move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
				let _span = tracing::trace_span!("audio provider callback").entered();

				let mut provider_maybe = stream_shared.provider.lock().unwrap();
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
				stream_shared.device_lost.store(true, Ordering::Relaxed);
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

#[instrument]
pub fn enumerate_audio_devices() -> anyhow::Result<()> {
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