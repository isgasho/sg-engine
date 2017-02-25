extern crate winit;
extern crate vulkano;
extern crate time;
extern crate cgmath;

extern crate engine;
use engine::renderer::{Vertex, Renderer};
use engine::libloader::LibLoader;

use cgmath::Vector3;

extern crate game_state;
use std::time::Duration;
use game_state::{state};

use std::thread;

// Platform-specific wiring for simulation and simulation2 dynamically loaded libs (hot loaded)
#[cfg(target_os = "windows")] const SIM_LIBPATH: &'static str = "mod_simulation/target/debug/mod_simulation.dll";
#[cfg(target_os = "linux")] const SIM_LIBPATH: &'static str = "mod_simulation/target/debug/libmod_simulation.so";

fn main() {
	// spin off the dylib loader in the background,
	// pretty useless for now but shows basic functionality
	//thread::spawn(|| {
	play_dylib_load();
	//});

	// draw with Renderer / Vulkano
	play_draw_stuff();
}


fn play_dylib_load() {
	let mut state = state::State { blob: 42, name: "(:S)".to_string(), data: vec!["arf".to_string()] };

	// because of #[no_mangle], each library needs it's own unique method name as well... sigh
	let mut sim = LibLoader::new(SIM_LIBPATH, "simulation");
	loop {
			thread::sleep(Duration::from_millis(1000));
			sim.check_update(&mut state);
			sim.tick(&mut state);
	}
}

fn play_draw_stuff() {
	let mut renderer = Renderer::new();

	let red  = [1.0, 0.0, 0.0, 1.0];
	let blue = [0.0, 1.0, 0.0, 1.0];
	let green= [0.0, 0.0, 1.0, 1.0];
	let items = vec![
		Vertex::new([-0.5, -0.25, 0.0], red),
		Vertex::new([0.0, 0.5, 0.0], green),
		Vertex::new([0.25, -0.1, 0.0], blue),

		Vertex::new([0.5, 0.25, 0.0], red),
		Vertex::new([0.0, -0.5, 0.0], blue),
		Vertex::new([-0.25, 0.1, 0.0], green),
	];

	let vertex_buffer = renderer.create_buffer(items);
	
	let mut frame = 0;
	'running: loop {
		frame += 1;
		if frame % 100 == 0 {
			println!("FPS: {}", renderer.fps());
		}

		thread::sleep(Duration::from_millis(16));
		renderer.render(&vertex_buffer);

		// Make use of winit
		for ev in renderer.native_window().poll_events() {
			match ev {
				winit::Event::Closed => {
					println!("Window closed.");
					break 'running;
				},
				_ => ()
			}
		}
	}
}

