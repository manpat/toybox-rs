# Toybox
[![](https://github.com/manpat/toybox-rs/actions/workflows/build-and-deploy-docs.yml/badge.svg)](https://manpat.github.io/toybox-rs)

A collection of things for making the making of things less bad for me and me alone.

Toybox based projects normally look something like this:
```rust no_run
use toybox::prelude::*;

let mut engine = toybox::Engine::new("your window name here")?;

'main: loop {
	engine.process_events();
	if engine.should_quit() {
		break 'main
	}

	// Update logic here

	let mut gfx = engine.gfx.render_state();
	gfx.set_clear_color(Color::grey(0.1));
	gfx.clear(gfx::ClearMode::ALL);

	// Render logic here

	engine.end_frame();
}

# Ok::<_, Box<dyn Error>>(())
```

## Profiling

Enable the `tracy` feature in your Cargo.toml if you want to profile things.