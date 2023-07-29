pub mod tile;

use std::{mem::MaybeUninit, collections::HashMap};

use ash::vk;
use rand::Rng;

use crate::engine::Engine;
use self::tile::{Tile, TileState};

pub struct Game {
    tiles: HashMap<[i8; 2], Tile>,
    current_direction: Direction,
    head: [i8; 2],
    tail: ([i8; 2], Direction),
    /// this is only here so that the tail knows where to turn
    directions: Vec<Direction>
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Direction {
    North,
    South,
    West,
    East
}

impl Game {
    pub fn new(engine: &Engine) -> Game {
        let mut tiles = HashMap::with_capacity(100);

        for x in 0..10i8 {
            for y in 0..10i8 {
                tiles.insert([x, y], Tile::new(
                    [x as f32 * 80.0, y as f32 * 80.0],
                    TileState::Empty,
                    engine
                ));
            }
        }

        tiles.get_mut(&[2, 5]).unwrap().tile_state = TileState::Snake;
        tiles.get_mut(&[3, 5]).unwrap().tile_state = TileState::Snake;
        tiles.get_mut(&[4, 5]).unwrap().tile_state = TileState::Snake;

        tiles.get_mut(&[7, 5]).unwrap().tile_state = TileState::Apple;

        Game {
            tiles,
            current_direction: Direction::East,
            head: [4, 5],
            tail: ([2, 5], Direction::East),
            directions: Vec::new()
        }
    }

    pub fn input(&mut self, direction: Direction) {
        let direction = match direction {
            Direction::North if self.current_direction != Direction::South => Direction::North,
            Direction::South if self.current_direction != Direction::North => Direction::South,
            Direction::West if self.current_direction != Direction::East => Direction::West,
            Direction::East if self.current_direction != Direction::West => Direction::East,
            _ => {self.current_direction}
        };

        self.current_direction = direction;

        if self.tail.1 != direction {
            if let Some(last_direction) = self.directions.last() {
                if last_direction != &direction {
                    self.directions.push(direction);
                }
            } else {
                self.directions.push(direction);
            }
        }
    }

    pub fn tick(&mut self) {
        let mut ate_apple = false;

        let forward = match self.current_direction {
            Direction::North => [0, 1],
            Direction::South => [0, -1],
            Direction::West => [-1, 0],
            Direction::East => [1, 0]
        };

        let head_pos = self.head;

        if let Some(forward_tile) = self.tiles.get_mut(&[
            head_pos[0] + forward[0],
            head_pos[1] + forward[1]
        ]) {
            match forward_tile.tile_state {
                TileState::Empty => {}
                TileState::Apple => {
                    ate_apple = true;
                }
                TileState::Snake => {
                    panic!("hit snake");
                }
            }

            // this should be only accessible if the tile is empty or apple
            forward_tile.tile_state = TileState::Snake;
        } else {
            panic!("hit border");
        }

        let tail_pos = self.tail.0;

        let tail_forward = match self.tail.1 {
            Direction::North => [0, 1],
            Direction::South => [0, -1],
            Direction::West => [-1, 0],
            Direction::East => [1, 0]
        };

        if !ate_apple {
            if let Some(forward_tile) = self.tiles.get(&[
                tail_pos[0] + tail_forward[0],
                tail_pos[1] + tail_forward[1]
            ]) {
                match forward_tile.tile_state {
                    TileState::Empty => {
                        self.tail.1 = self.directions.remove(0);
                    }
                    TileState::Apple => {
                        self.tail.1 = self.directions.remove(0);
                    }
                    TileState::Snake => {
                    }
                }
            } else {
                self.tail.1 = self.directions.remove(0);
            }

            self.tiles.get_mut(&[tail_pos[0], tail_pos[1]]).unwrap().tile_state = TileState::Empty;
        }

        let tail_forward = match self.tail.1 {
            Direction::North => [0, 1],
            Direction::South => [0, -1],
            Direction::West => [-1, 0],
            Direction::East => [1, 0]
        };

        self.head = [head_pos[0] + forward[0], head_pos[1] + forward[1]];
        if !ate_apple {
            self.tail.0 = [tail_pos[0] + tail_forward[0], tail_pos[1] + tail_forward[1]];
        }

        if ate_apple {
            loop {
                let rand_x: i8 = rand::thread_rng().gen_range(0..10);
                let rand_y: i8 = rand::thread_rng().gen_range(0..10);

                if self.tiles.get(&[rand_x, rand_y]).unwrap().tile_state == TileState::Empty {
                    self.tiles.get_mut(&[rand_x, rand_y]).unwrap().tile_state = TileState::Apple;
                    break;
                }
            }
        }
    }

    pub fn draw(&self, draw_command_buffer: vk::CommandBuffer, device: &ash::Device, pipeline_layout: vk::PipelineLayout) {
        for x in 0..10 {
            for y in 0..10 {
                self.tiles[&[x, y]].draw(draw_command_buffer, device, pipeline_layout);
            }
        }
    }
}
