use std::{array, num::NonZeroUsize};

use ndarray::{arr3, Array3, Axis, Array2, arr2};
use noise::{SuperSimplex, NoiseFn};
use ultraviolet::{IVec2, Vec2, Vec3, IVec3};

use crate::world::{block_data::{StaticBlockData, BlockHandle}, section::Section};

use super::noise::{ScaleNoise2D, ScaleNoise3D};

pub struct TerrainGenerator {
    pub planar_noise: ScaleNoise2D,
    pub world_noise: ScaleNoise3D,
    pub overall_height: ScaleNoise2D,
    cache: [BlockHandle; 5]
}

impl TerrainGenerator {
    const NOISE_SCALE: f32 = 0.01;
    const OVERALL_SCALE: f32 = 0.001;

    pub fn new(seed: u32, block_data: &StaticBlockData) -> Self {
        let planar_noise = ScaleNoise2D::new(
            Vec2::new(Self::NOISE_SCALE, Self::NOISE_SCALE), 
            seed
        );
        let world_noise = ScaleNoise3D::new(
            Vec3::new(Self::NOISE_SCALE, Self::NOISE_SCALE, Self::NOISE_SCALE), 
            seed
        );

        let overall_height = ScaleNoise2D::new(
            Vec2::new(Self::OVERALL_SCALE, Self::OVERALL_SCALE), 
            seed
        );

        let cache = [
            block_data.get_handle("air").unwrap(),
            block_data.get_handle("grass").unwrap(),
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
        ((self.planar_noise.get(pos.into()) + 1.0) * 20.0) as u32
    }

    pub fn gen_at(&self, pos: Vec3) -> BlockHandle {
        let m = self.world_noise.get(pos) as f32;
        if m >= 0.9 {
            return self.cache[4]
        } else if m >= 0.30 {
            return self.cache[3]
        } else if m >= 0.15 {
            return self.cache[2]
        } else if m >= 0.10 {
            return self.cache[1]
        } else {
            return self.cache[0]
        }
    }

    pub fn gen_chunk(&self, chunk_pos: IVec2) {
        
    }

    // ((height as f32 - pos.y) / 20.0).clamp(-1.0, 1.0)
    pub fn gen_section(&self, offset: IVec3) -> Section {
        let mut arr = Array3::from_elem((16, 16, 16), BlockHandle::default());
        for (i, mut column) in arr.lanes_mut(Axis(1)).into_iter().enumerate() {
            let xz_off = IVec2::new((i / 16) as i32, (i % 16) as i32);
            let xz = IVec2::new(offset.x + xz_off.x, offset.z + xz_off.y);
            let height = self.height_modifier(xz.into());
            let flatness = self.flatness_modifier(xz.into());
            for (y_usize, block) in column.iter_mut().enumerate().rev() {
                let y_off = y_usize as i32;
                let section_offset = IVec3::new(xz_off.x, y_off, xz_off.y);
                let pos = Vec3::from(offset + section_offset);
                let m = self.world_noise.get(pos) as f32 + ((height - pos.y) / (flatness * 10.0)).clamp(-1.0, 1.0);

                let pos = pos + (Vec3::unit_y() * 4.0);
                let m4 = self.world_noise.get(pos) as f32 + ((height - pos.y) / (flatness * 10.0)).clamp(-1.0, 1.0);

                *block = {
                    if m >= 1.0 || m4 >= 0.15 {
                        self.cache[4]
                    } else if m >= 0.30 {
                        self.cache[3]
                    } else if m >= 0.15 {
                        self.cache[2]
                    } else if m >= 0.10 {
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

    fn height_modifier(&self, pos: Vec2) -> f32 {
        (self.planar_noise.get(pos).powi(2) * 30.0) as f32 + 50.0
    }

    fn flatness_modifier(&self, pos: Vec2) -> f32 {
        ((self.overall_height.get(pos) + 1.0) * 0.5) as f32
    }
}

struct ChunkHeightSampler {
    height_data: Array2<f32>,
}

impl ChunkHeightSampler {
    pub fn new(offset: Vec2, res: NonZeroUsize, noise: ScaleNoise2D, octaves: u32) -> Self {
        let res = res.get();
        let step_size = 16.0 / res as f32;
        let arr = Array2::from_shape_fn((res + 1, res + 1), 
            |(step_x, step_y)| {
                let x = offset.x + (step_x as f32 * step_size);
                let y = offset.y + (step_y as f32 * step_size);
                noise.sample(Vec2::new(x, y), octaves) as f32
            }
        );

        Self {
            height_data: arr,
        }
    }

    pub fn sample(&self, relative_pos: Vec2) -> f32 {
        if relative_pos.component_max() > 16.0 || relative_pos.component_min() < 0.0 {
            panic!("Invalid sample position: {:?}", relative_pos);
        }

        let res = self.height_data.len_of(Axis(0));
        let mul = res as f32 / 16.0;

        let rounded_x = (relative_pos.x * mul).floor();
        let rounded_y = (relative_pos.y * mul).floor();
        let data_x = rounded_x as usize;
        let data_y = rounded_y as usize;

        let x0 = rounded_x * res as f32;
        let y0 = rounded_y * res as f32;
        let x1 = (rounded_x + 1.0) * res as f32;
        let y1 = (rounded_y + 1.0) * res as f32;

        let p00 = self.height_data[(data_x, data_y)];
        let p10 = self.height_data[(data_x + 1, data_y)];
        let p01 = self.height_data[(data_x, data_y + 1)];
        let p11 = self.height_data[(data_x + 1, data_y + 1)];

        let precomp = (
            (relative_pos.x - x0) / (x1 - x0), 
            (relative_pos.y - y0) / (y1 - y0)
        );

        p00 + 
            ((p10 - p00) * precomp.0) +
            ((p01 - p00) * precomp.1) + 
            ((p11 - p01 - p10 + p00) * precomp.0 * precomp.1)
    }
}