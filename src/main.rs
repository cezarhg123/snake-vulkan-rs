pub mod engine;
pub mod game;

use std::time::Instant;
use engine::Engine;
use game::{tile::Tile, Game, Direction};
use glfw::{Key, Action};

fn main() {
    let mut engine = Engine::new();

    let mut game = Game::new(&engine);

    let mut accept_input = true;
    let mut prev_time = Instant::now();
    while engine.running() {
        if accept_input {   
            let window = engine.window();
            if window.get_key(Key::W) == Action::Press {
                game.input(Direction::North);
                accept_input = false;
            } else if window.get_key(Key::S) == Action::Press {
                game.input(Direction::South);
                accept_input = false;
            } else if window.get_key(Key::A) == Action::Press {
                game.input(Direction::West);
                accept_input = false;
            } else if window.get_key(Key::D) == Action::Press {
                game.input(Direction::East);
                accept_input = false;
            }
        }
        
        let crnt_time = Instant::now();
        if (crnt_time - prev_time).as_secs_f32() >= 1.0 / 10.0 {
            game.tick();
            prev_time = crnt_time;
            accept_input = true;
        }

        engine.begin_draw();
        let draw_command_buffer = engine.draw_command_buffer();
        let device = engine.device();
        let pipeline_layout = engine.pipeline_layout();
        game.draw(draw_command_buffer, &device, pipeline_layout);
        engine.end_draw();
    }

    unsafe {
        engine.device().device_wait_idle().unwrap();
    }
}
