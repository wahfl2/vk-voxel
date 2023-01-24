use noise::{SuperSimplex, NoiseFn};
use ultraviolet::{IVec2, Vec2};

use super::block_data::BlockHandle;

pub struct TerrainGenerator {
    pub fill_block: BlockHandle,
    pub noise: ScaleNoise,
}

impl TerrainGenerator {
    const NOISE_SCALE: f32 = 0.01;

    pub fn new(seed: u32) -> Self {
        let noise = ScaleNoise::new(Vec2::new(Self::NOISE_SCALE, Self::NOISE_SCALE), seed);
        Self { noise, fill_block: BlockHandle::new_unsafe(1) }
    }

    pub fn new_random() -> Self {
        let seed = rand::random::<u32>();
        Self::new(seed)
    }

    pub fn get_height(&self, pos: IVec2) -> u32 {
        ((self.noise.get(pos.into()) + 1.0) * 20.0) as u32
    }
}

impl Default for TerrainGenerator {
    fn default() -> Self {
        Self::new(0)
    }
}

pub struct ScaleNoise {
    scale: Vec2,
    noise: SuperSimplex,
}

impl ScaleNoise {
    pub fn new(scale: Vec2, seed: u32) -> Self {
        Self {
            scale,
            noise: SuperSimplex::new(seed)
        }
    }

    pub fn get(&self, pos: Vec2) -> f64 {
        let scaled = pos * self.scale;
        self.noise.get([scaled.x as f64, scaled.y as f64])
    }
}