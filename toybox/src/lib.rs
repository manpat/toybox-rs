#![doc = include_str!("../README.md")]

pub mod prelude;
pub use crate::prelude::*;


pub trait App {
	fn present(&mut self /*, _: Context*/);
}


// pub fn run<F, M>(start_main_loop: F) -> anyhow::Result<()>
//     where M: MainLoop + 'static
//         , F: FnOnce() -> anyhow::Result<M>
// {

//     let mut main_loop = start_main_loop()?;

//     event_loop.run(move |event, _, control_flow| {
//         control_flow.set_poll();

//         match event {
//             Event::WindowEvent { event: WindowEvent::CloseRequested, .. }
//             | Event::DeviceEvent {
//                 event: DeviceEvent::Key(KeyboardInput{ virtual_keycode: Some(VirtualKeyCode::Escape), .. }), .. } => {
//                 control_flow.set_exit();
//             }

//             Event::MainEventsCleared => {
//                 main_loop.present();
//                 surface.swap_buffers(&context).unwrap();
//             }

//             Event::WindowEvent { event: WindowEvent::Resized(physical_size), .. } => {
//                 main_loop.resize(Vec2i::new(physical_size.width as i32, physical_size.height as i32));
//             }

//             _ => {}
//         }
//     });
// }


