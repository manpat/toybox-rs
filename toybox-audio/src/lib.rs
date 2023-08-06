use cpal::traits::*;

pub mod prelude {

}


pub fn init() -> anyhow::Result<System> {
	let host = cpal::default_host();
	let device = host.default_output_device().expect("no output device available");

	let supported_configs_range = device.supported_output_configs()
		.expect("error while querying configs");

	let supported_config = supported_configs_range
		.max_by(cpal::SupportedStreamConfigRange::cmp_default_heuristics)
		.expect("no supported config?!")
		.with_sample_rate(cpal::SampleRate(44100));

	dbg!(&supported_config);

	let config = supported_config.into();

	let mut t = 0.0f32;

	let stream = device.build_output_stream(
		&config,
		move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
			// react to stream events and read or write stream data here.
			// for s in data.iter_mut() {
			// 	if ((t/4.0).fract() < 0.5) {
			// 		let ph = t * 220.0 * 9.0/8.0 * std::f32::consts::PI;
			// 		*s = ph.sin() + (ph * 3.0/2.0).sin() + (ph * 6.0/5.0).sin() + (ph * 9.0/5.0).sin();
			// 	} else {
			// 		let ph = t * 220.0 * std::f32::consts::PI;
			// 		*s = ph.sin() + (ph * 3.0/2.0).sin() + (ph * 5.0/4.0).sin() + (ph * 15.0/8.0).sin() + (ph * 9.0/8.0*2.0).sin();
			// 	}

			// 	*s /= 5.0;
			// 	t += 1.0 / 44100.0;
			// }
		},
		move |err| {
			// react to errors here.
		},
		None // None=blocking, Some(Duration)=timeout
	)?;

	// stream.play()?;

	Ok(System {
		stream
	})
}

pub struct System {
	stream: cpal::Stream,
}