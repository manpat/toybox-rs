use cpal::traits::*;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};


pub mod prelude {
	pub use super::Provider;
}


pub fn init() -> anyhow::Result<System> {
	let host = cpal::default_host();

	if false {
		println!("vvvvvvv Available audio devices vvvvvvvv");

		for device in host.output_devices()? {
			println!("   => {}", device.name()?);
			println!("      => default output config: {:?}", device.default_output_config()?);
			println!("      => supported configs:");
			for config in device.supported_output_configs()? {
				println!("         => {config:?}");
			}
			println!();
		}

		println!("^^^^^^ Available audio devices ^^^^^^^");
	}

	let shared = Arc::new(SharedState {
		provider: Mutex::new(None),
		device_lost: AtomicBool::new(false),
	});

	let (stream, current_configuration) = build_output_stream(&host, &shared)?;

	Ok(System {
		host,
		stream: Some(stream),

		current_configuration,
		shared,
	})
}

pub struct System {
	host: cpal::Host,
	stream: Option<cpal::Stream>,
	current_configuration: Configuration,

	shared: Arc<SharedState>,
}

impl System {
	pub fn update(&mut self) {
		if self.shared.device_lost.load(Ordering::Relaxed) {
			if self.stream.is_some() {
				self.stream = None;
			} else {
				// If device_lost and there's no existing stream, then we've given up on creating a device for now
				return;
			}
		}

		if self.stream.is_some() {
			return;
		}

		let Ok((stream, current_configuration)) = build_output_stream(&self.host, &self.shared) else {
			println!("Failed to create audio device");
			return;
		};

		self.stream = Some(stream);
		self.current_configuration = current_configuration;
		self.shared.device_lost.store(false, Ordering::Relaxed);
	}
}

impl System {
	pub fn set_provider<P>(&mut self, provider: impl Into<Option<P>>) -> anyhow::Result<()>
		where P : Provider + Send
	{
		let mut shared_provider = self.shared.provider.lock().unwrap();

		if let Some(mut provider) = provider.into() {
			provider.on_configuration_changed(self.current_configuration);
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
	fn on_configuration_changed(&mut self, _: Configuration);
	fn fill_buffer(&mut self, buffer: &mut [f32]);
}


struct SharedState {
	provider: Mutex<Option<Box<dyn Provider>>>,
	device_lost: AtomicBool,
}


// should be able to close and reopen streams dynamically, potentially on different devices
//	any non-device state should be maintained
// 	should be able to cope with different sample rates


fn build_output_stream(host: &cpal::Host, shared: &Arc<SharedState>) -> anyhow::Result<(cpal::Stream, Configuration)> {
	let device = host.default_output_device().expect("no output device available");

	let supported_configs_range = device.supported_output_configs()
		.expect("error while querying configs");

	// TODO(pat.m): support different sample formats
	let supported_config = supported_configs_range
		.filter(|config| config.sample_format().is_float())
		.max_by(cpal::SupportedStreamConfigRange::cmp_default_heuristics)
		.expect("no supported config?!");

	let desired_sample_rate = 48000.clamp(supported_config.min_sample_rate().0, supported_config.max_sample_rate().0);
	let supported_config = supported_config
		.with_sample_rate(cpal::SampleRate(desired_sample_rate));

	dbg!(&supported_config);

	let config = supported_config.into();

	let stream = device.build_output_stream(
		&config,
		{
			let shared = Arc::clone(&shared);

			move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
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
				println!("audio device error! {err}");
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

	Ok((stream, configuration))
}