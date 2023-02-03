use ndarray::{arr3, Array3, Axis};
use noise::{SuperSimplex, NoiseFn};
use ultraviolet::{IVec2, Vec2, Vec3, IVec3};

use crate::util::util::AdditionalSwizzles;

use super::{block_data::{BlockHandle, StaticBlockData}, section::Section};

pub struct TerrainGenerator {
    pub planar_noise: ScaleNoise,
    pub world_noise: ScaleNoise,
    pub overall_height: ScaleNoise,
    cache: [BlockHandle; 4]
}

impl TerrainGenerator {
    const NOISE_SCALE: f32 = 0.02;
    const OVERALL_SCALE: f32 = 0.005;

    pub fn new(seed: u32, block_data: &StaticBlockData) -> Self {
        let planar_noise = ScaleNoise::new(
            Vec3::new(Self::NOISE_SCALE, Self::NOISE_SCALE, 0.0), 
            seed
        );
        let world_noise = ScaleNoise::new(
            Vec3::new(Self::NOISE_SCALE, Self::NOISE_SCALE, Self::NOISE_SCALE), 
            seed
        );

        let overall_height = ScaleNoise::new(
            Vec3::new(Self::OVERALL_SCALE, Self::OVERALL_SCALE, 0.0), 
            seed
        );

        let cache = [
            block_data.get_handle("air").unwrap(),
            block_data.get_handle("grass_block").unwrap(),
            block_data.get_handle("dirt").unwrap(),
            block_data.get_handle("stone").unwrap(),
        ];

        Self { planar_noise, world_noise, overall_height, cache }
    }

    pub fn new_random(block_data: &StaticBlockData) -> Self {
        let seed = rand::random::<u32>();
        Self::new(seed, block_data)
    }

    pub fn get_height(&self, pos: IVec2) -> u32 {
        ((self.planar_noise.get_2d(pos.into()) + 1.0) * 20.0) as u32
    }

    pub fn gen_at(&self, pos: Vec3) -> BlockHandle {
        let m = self.world_noise.get_3d(pos) as f32;
        if m >= 0.9 {
            return self.cache[3]
        } else if m >= 0.30 {
            return self.cache[2]
        } else if m >= 0.15 {
            return self.cache[1]
        } else {
            return self.cache[0]
        }
    }
    // ((height as f32 - pos.y) / 20.0).clamp(-1.0, 1.0)
    pub fn gen_section(&self, offset: IVec3) -> Section {
        let mut arr = Array3::from_elem((16, 16, 16), BlockHandle::default());
        for (i, mut column) in arr.lanes_mut(Axis(1)).into_iter().enumerate() {
            let x_off = (i / 16) as i32;
            let z_off = (i % 16) as i32;
            let height = self.height_multiplier(IVec2::from((offset.x + x_off, offset.z + z_off)).into());
            for (y_usize, block) in column.iter_mut().enumerate() {
                let y_off = y_usize as i32;
                let section_offset = IVec3::new(x_off, y_off, z_off);
                let pos = Vec3::from(offset + section_offset);
                let m = self.world_noise.get_3d(pos) as f32 + ((height - pos.y) / 20.0).clamp(-1.0, 1.0);

                *block = {
                    if m >= 0.9 {
                        self.cache[3]
                    } else if m >= 0.30 {
                        self.cache[2]
                    } else if m >= 0.15 {
                        self.cache[1]
                    } else {
                        self.cache[0]
                    }
                }
            }
        }
        
        Section {
            blocks: arr,
            ..Section::empty()
        }
    }

    fn height_multiplier(&self, pos: Vec2) -> f32 {
        let flatness = self.overall_height.get_2d(pos);
        (self.planar_noise.get_2d(pos).powi(2) * (flatness * 30.0) + 50.0) as f32
    }
}

pub struct ScaleNoise {
    scale: Vec3,
    noise: SuperSimplex,
}

impl ScaleNoise {
    pub fn new(scale: Vec3, seed: u32) -> Self {
        Self {
            scale,
            noise: SuperSimplex::new(seed)
        }
    }

    pub fn get_2d(&self, pos: Vec2) -> f64 {
        let scaled = pos * self.scale.xy();
        self.noise.get([scaled.x as f64, scaled.y as f64])
    }

    pub fn get_3d(&self, pos: Vec3) -> f64 {
        let scaled = pos * self.scale;
        self.noise.get([scaled.x as f64, scaled.y as f64, scaled.z as f64])
    }
}