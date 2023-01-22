use noise::{SuperSimplex, NoiseFn};
use ultraviolet::IVec2;

use super::block_data::BlockHandle;

pub struct TerrainGenerator {
    pub fill_block: BlockHandle,
    pub noise: SuperSimplex,
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> Self {
        let noise = SuperSimplex::new(seed);
        Self { noise, fill_block: BlockHandle::new_unsafe(1) }
    }

    pub fn new_random() -> Self {
        let seed = rand::random::<u32>();
        Self::new(seed)
    }

    pub fn get_height(&self, pos: IVec2) -> u32 {
        ((self.noise.get([pos.x as f64, pos.y as f64]) + 1.0) * 1.0) as u32
    }
}

impl Default for TerrainGenerator {
    fn default() -> Self {
        Self::new(0)
    }
}