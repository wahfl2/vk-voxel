use std::array;
use std::num::NonZeroUsize;

use ndarray::{arr1, arr2};
use ndarray::{Array3, Axis, Array2};
use rand_xoshiro::Xoshiro128StarStar;
use rand_xoshiro::rand_core::{SeedableRng, RngCore};
use turborand::TurboRand;
use turborand::rng::Rng;
use ultraviolet::{IVec2, Vec2, Vec3, IVec3};

use crate::world::chunk::Chunk;
use crate::world::{block_data::{StaticBlockData, BlockHandle}, section::Section};

use super::noise::{ScaleNoise2D, ScaleNoise3D};

pub struct TerrainGenerator {
    pub planar_noise: ScaleNoise2D,
    pub world_noise: ScaleNoise3D,
    pub overall_height: ScaleNoise2D,
    pub seed: u32,
    rng: Xoshiro128StarStar,
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

        let rng = Xoshiro128StarStar::seed_from_u64(seed as u64);

        let cache = [
            block_data.get_handle("air").unwrap(),
            block_data.get_handle("grass").unwrap(),
            block_data.get_handle("grass_block").unwrap(),
            block_data.get_handle("dirt").unwrap(),
            block_data.get_handle("stone").unwrap(),
        ];

        Self { planar_noise, world_noise, overall_height, rng, seed, cache }
    }

    pub fn new_random(block_data: &StaticBlockData) -> Self {
        let seed = Rng::new().u32(..);
        Self::new(seed, block_data)
    }

    pub fn gen_chunk(&mut self, chunk_pos: IVec2) -> Chunk {
        let height_sampler = ChunkHeightSampler::new(
            (chunk_pos * 16).into(), 
            NonZeroUsize::new(8).unwrap(), 
            &self.planar_noise, 
            4
        );

        let off = Vec2::new(0.5, 0.5);
        let mut lowest = 999;
        let mut highest = 0;
        let height_array = Array2::from_shape_fn((16, 16), |(x_step, y_step)| {
            let pos = off + Vec2::new(x_step as f32, y_step as f32);
            let height = (height_sampler.sample(pos) * 50.0 + 50.0).round() as u32;
            let low_gen = height.saturating_sub(4);
            if low_gen < lowest { lowest = low_gen; }
            if height > highest { highest = height; }

            height
        });

        let section_low = lowest / 16;
        let section_high = highest / 16;

        let sections = Box::new(array::from_fn(|i| {
            let idx = i as u32;

            if idx < section_low {
                Section::full(self.cache[4])
            } else if idx <= section_high {
                self.section_from_height(&height_array, i as u32, chunk_pos)
            } else {
                Section::full(self.cache[0])
            }            
        }));

        Chunk {
            pos: chunk_pos,
            sections,
        }
    }

    fn section_from_height(&mut self, height_array: &Array2<u32>, section_num: u32, chunk_pos: IVec2) -> Section {
        let height_offset = section_num * 16;
        let mut ret = Section::empty();
        for (i, mut column) in ret.blocks.lanes_mut(Axis(1)).into_iter().enumerate() {
            let (x, y) = ((i / 16), (i % 16));
            let column_pos = (chunk_pos * 16) + IVec2::new(x as i32, y as i32);
            let height = height_array[(x, y)];
            let relative_height = height.saturating_sub(height_offset);

            // All air
            if height < height_offset { continue; }

            // All stone
            if height > height_offset + 20 {
                column.fill(self.cache[4]);
                continue;
            }

            let can_gen_grass = height >= height_offset.saturating_sub(1) && relative_height < 15;
            let stone_end = relative_height.saturating_sub(3).min(15) as usize;
            let dirt_end = relative_height.min(15) as usize;
            let mut c = [self.cache[0]; 16];

            c[0..stone_end].fill(self.cache[4]);
            c[stone_end..dirt_end].fill(self.cache[3]);
            c[relative_height.min(15) as usize] = self.cache[2];

            let grass_pos = IVec3::new(
                column_pos.x, 
                ((section_num * 16) + relative_height + 1) as i32, 
                column_pos.y
            );

            if can_gen_grass && self.gen_grass_at(grass_pos) {
                c[relative_height as usize + 1] = self.cache[1];
            }

            column.assign(&arr1(&c));
        }
        ret
    }

    fn gen_grass_at(&mut self, pos: IVec3) -> bool {
        self.rng.next_u32() & 1 == 0
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
    pub fn new(offset: Vec2, res: NonZeroUsize, noise: &ScaleNoise2D, octaves: u32) -> Self {
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

    /// Bi-linear interpolation of the input data
    pub fn sample(&self, relative_pos: Vec2) -> f32 {
        if relative_pos.component_max() > 16.0 || relative_pos.component_min() < 0.0 {
            panic!("Invalid sample position: {:?}", relative_pos);
        }

        let res = (self.height_data.len_of(Axis(0)) - 1) as f32;
        let mul = res / 16.0;

        let rounded_x = (relative_pos.x * mul).floor();
        let rounded_y = (relative_pos.y * mul).floor();
        let data_x = rounded_x as usize;
        let data_y = rounded_y as usize;

        let recip = mul.recip();
        let x0 = rounded_x * recip;
        let y0 = rounded_y * recip;
        let x1 = (rounded_x + 1.0) * recip;
        let y1 = (rounded_y + 1.0) * recip;

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
            ((p11 + p00 - p01 - p10) * precomp.0 * precomp.1)
    }
}

#[test]
fn height_sampler_test() {
    let sampler = ChunkHeightSampler {
        height_data: arr2(&[[0.0, 0.0], [1.0, 1.0]])
    };

    assert_eq!(sampler.sample(Vec2::new(8.0, 4.9743)), 0.5);
}