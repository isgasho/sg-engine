extern crate time;

extern crate game_state;

#[macro_use]
extern crate engine;
use engine::libloader::LibLoader;

use std::time::Duration;
use game_state::state;

use std::thread;

use game_state::state::WindowAccess;

fn main() {
    let mut state = state::State::new();


    // TODO mod_audio
    // TODO mod_gui
    // TODO mod_network

    // because of #[no_mangle], each library needs it's own unique method name as well... sigh

    let mut mods = Vec::new();
    mods.push(load_mod!(input));
    mods.push(load_mod!(simulation));

    state.add_window(800, 600, "sg-shell 1 (vulkan)".to_string());
    mods.push(load_mod!(rendering_vulkan));

    //state.add_window_builder(800, 600, "sg-shell (OpenGL/glutin)".to_string());
    //mods.push(load_mod!(rendering_opengl));

    mods.push(load_mod!(asset_loader));

    for mut m in mods.iter_mut() {
        m.check_update(&mut state);
    }

    let mut frame = 0;
    let frame_budget = 16000i64;// for 60 fps
    let mut last_update = time::now();

    loop {
        // TODO: gather delta time instead

        let mut total_time = 0i64;
        for m in mods.iter() {
            let mut start_update = time::now();
            let duration: time::Duration = m.update(&mut state, &(start_update - last_update));
            if frame % 300 == 0 {
                print!(
                    "|> {}: {total_time:>6} μs ",
                    name=m.get_name(),
                    total_time=duration.num_microseconds().unwrap_or(0)
                );
            }
            total_time += duration.num_microseconds().unwrap_or(0);
        }
        last_update = time::now();
        if frame % 300 == 0 {
            println!("|>= total time: {total_time:>6} μs", total_time = total_time);
        }
        if frame % 30 == 0 {
            for mut m in mods.iter_mut() {
                m.check_update(&mut state);
            }
        }
        frame += 1;

        let wait = (frame_budget - total_time) / 1000;
        if wait > 0 {
            thread::sleep(Duration::from_millis(wait as u64));
        }

    }
}

