use rustc_data_structures::stable_map::FxHashMap;
use ultraviolet::{IVec2, Vec2};

use crate::render::{renderer::Renderer, texture::TextureAtlas};

use super::{chunk::Chunk, terrain::TerrainGenerator, block_data::StaticBlockData};

pub struct World {
    pub loaded_chunks: FxHashMap<IVec2, Chunk>,
    pub terrain_generator: TerrainGenerator,
    pub player_pos: Vec2,
}

impl World {
    const CHUNK_UPDATES_PER_FRAME: u32 = 4;
    const RENDER_DISTANCE: u32 = 5;

    pub fn new() -> Self {
        Self {
            loaded_chunks: FxHashMap::default(),
            terrain_generator: TerrainGenerator::new_random(),
            player_pos: Vec2::zero(),
        }
    }

    pub fn load_chunk(&mut self, chunk_pos: IVec2, atlas: &TextureAtlas, block_data: &StaticBlockData) {
        // TODO: Load from storage
        let mut new_chunk = Chunk::empty(chunk_pos);
        new_chunk.gen(&self.terrain_generator);
        new_chunk.rebuild_mesh(atlas, block_data);
        self.loaded_chunks.insert(chunk_pos, new_chunk);
        println!("Loaded ({}, {})", chunk_pos.x, chunk_pos.y);
    }

    pub fn frame_update(&mut self, renderer: &mut Renderer, block_data: &StaticBlockData) {
        for _ in 0..Self::CHUNK_UPDATES_PER_FRAME {
            if let Some(pos) = self.get_closest_unloaded_chunk() {
                self.load_chunk(pos, &renderer.texture_atlas, block_data);
                renderer.upload_chunk(pos, self.loaded_chunks.get(&pos).unwrap(), block_data);
            } else {
                break
            }
        }
    }

    fn get_closest_unloaded_chunk(&self) -> Option<IVec2> {
        let div_16 = self.player_pos / 16.0;
        let center_chunk = IVec2::new(div_16.x.floor() as i32, div_16.y.floor() as i32);
        
        let mut check = center_chunk.clone();
        let mut step = SpiralStep::Right;
        let mut steps_left = 1;
        let mut step_amount = 1;
        let mut up_step = false;

        while let Some(_) = self.loaded_chunks.get(&check) {
            match step {
                SpiralStep::Right => check.x += 1,
                SpiralStep::Up => check.y += 1,
                SpiralStep::Left => check.x -= 1,
                SpiralStep::Down => check.y -= 1,
            }

            if (check - center_chunk).mag() as u32 > Self::RENDER_DISTANCE {
                return None
            }

            steps_left -= 1;
            if steps_left == 0 {
                if up_step { step_amount += 1; }
                up_step = !up_step;
                steps_left = step_amount;
                step.next();
            }
        }
        return Some(check);
    }
}

enum SpiralStep {
    Up,
    Down,
    Left,
    Right,
}

impl SpiralStep {
    fn next(&mut self) {
        match self {
            Self::Right => *self = Self::Up,
            Self::Up => *self = Self::Left,
            Self::Left => *self = Self::Down,
            Self::Down => *self = Self::Right,
        }
    }
}