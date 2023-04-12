use std::array;
use std::num::NonZeroUsize;

use ndarray::{arr1, s};
use ndarray::{Axis, Array2};
use noise::{SuperSimplex, NoiseFn};
use rand_xoshiro::Xoshiro128StarStar;
use rand_xoshiro::rand_core::{SeedableRng, RngCore};
use turborand::TurboRand;
use turborand::rng::Rng;
use ultraviolet::{IVec2, Vec2, Vec3, IVec3, UVec2, DVec3, UVec3};

use crate::util::more_vec::UsizeVec3;
use crate::util::util::{MoreCmp, VecRounding, UVecToSigned, MoreVecConstructors, AdditionalSwizzles};
use crate::world::block_data::Blocks;
use crate::world::chunk::Chunk;
use crate::world::{block_data::{StaticBlockData, BlockHandle}, section::Section};

use super::noise::{ScaleNoise2D, ScaleNoise3D};
use super::transformer::TerrainTransformer;

const CAVE_SAMPLES: usize = 500;

pub struct TerrainGenerator {
    pub planar_noise: ScaleNoise2D,
    pub world_noise: ScaleNoise3D,
    pub overall_height: ScaleNoise2D,
    pub seed: u32,
    cave_transformer: TerrainTransformer<[Vec3; CAVE_SAMPLES]>,
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

        Self {
            planar_noise, 
            world_noise, 
            overall_height, 
            seed,
            cave_transformer: Self::cave_transformer(seed),
            rng, cache
        }
    }

    fn cave_transformer(seed: u32) -> TerrainTransformer<[Vec3; CAVE_SAMPLES]> {
        TerrainTransformer::new(
            UVec2::new(20, 20),
            UVec2::new(7, 7),
            move |offset, size| {
                let seed_i = Self::basic_mix(seed, [offset.x as i64, offset.y as i64]);
                let seed: u64 = unsafe { std::mem::transmute(seed_i) };

                let mut rng = Xoshiro128StarStar::seed_from_u64(seed);
                let noise = SuperSimplex::new(rng.next_u32());
                let filler = DVec3::new(
                    6540960043.0,
                    6348425471.0,
                    9810259013.0,
                );

                let length = DVec3::one() * 30.0;
                let step = length / CAVE_SAMPLES as f64;
                const MULTIPLIER: Vec3 = Vec3::new(1.0, 0.5, 1.0);
                const MOVE_LENGTH: f32 = 4.0;

                let middle = Vec2::from(size) * 8.0;
                let mut worm_pos = Vec3::new(middle.x, 20.0, middle.y);

                array::from_fn::<Vec3, CAVE_SAMPLES, _>(|i| {
                    let d = i as f64 * step;
                    let movement = MOVE_LENGTH * (MULTIPLIER * Vec3::new(
                        noise.get([d.x, filler.y, filler.z]) as f32,
                        noise.get([filler.x, d.y, filler.z]) as f32,
                        noise.get([filler.x, filler.y, d.z]) as f32,
                    )).normalized();

                    worm_pos += movement;
                    worm_pos
                })
            },
            |chunk, offset, size, data| {
                const CAVE_RADIUS: f32 = 4.0;
                const RAD_SQ: f32 = CAVE_RADIUS * CAVE_RADIUS;
                let relative_pos = (chunk.blocks.pos - offset) * 16;

                let min_accept = Vec2::splat(CAVE_RADIUS);
                let max_accept = Vec2::from(size * 16) - Vec2::splat(CAVE_RADIUS);

                let min = relative_pos;
                let max = relative_pos + (IVec2::one() * 16);

                let min_chunk = Vec3::new(min.x as f32, 0.0, min.y as f32);
                let max_chunk = Vec3::new(max.x as f32, 256.0, max.y as f32);

                for carve in data.iter() {
                    if !(carve.xz().all_greater_than(&min_accept) && carve.xz().all_less_than(&max_accept)) {
                        continue;
                    }

                    let min_carve = (*carve - Vec3::one() * (CAVE_RADIUS + 1.0)).clamped(Vec3::zero(), Vec3::one() * 256.0).round();
                    let max_carve = (*carve + Vec3::one() * (CAVE_RADIUS + 1.0)).clamped(Vec3::zero(), Vec3::one() * 256.0).round();

                    if min_carve.any_greater_than(&max_chunk) || max_carve.any_less_than(&min_chunk) {
                        continue;
                    }

                    let min_section = ((min_carve.y / 16.0).floor() as u32).min(15);
                    let max_section = ((max_carve.y / 16.0).floor() as u32).min(15);

                    for i in min_section..=max_section {
                        let section = chunk.blocks.sections.get_mut(i as usize).unwrap();

                        let y_off = (i * 16) as f32;
                        let mn = min_carve - Vec3::new(min_chunk.x, y_off, min_chunk.z);
                        let mx = max_carve - Vec3::new(min_chunk.x, y_off, min_chunk.z);

                        let min_section = UVec3::new(mn.x as u32, mn.y as u32, mn.z as u32).clamped(UVec3::zero(), UVec3::one() * 16);
                        let max_section = UVec3::new(mx.x as u32, mx.y as u32, mx.z as u32).clamped(UVec3::zero(), UVec3::one() * 16);

                        let min_idx = UsizeVec3::from(min_section);
                        let max_idx = UsizeVec3::from(max_section);

                        section.blocks.slice_mut(s![
                            min_idx.x..max_idx.x, 
                            min_idx.y..max_idx.y, 
                            min_idx.z..max_idx.z,
                        ]).indexed_iter_mut().for_each(|(offset, block)| {
                            let offset = UsizeVec3::from(offset);
                            let block_pos = (offset + min_idx).into_vec3() + 
                                Vec3::new(min_chunk.x, y_off, min_chunk.z) + 
                                (0.5 * Vec3::one());

                            if (*carve - block_pos).mag_sq() <= RAD_SQ {
                                *block = Blocks::Air.handle();
                            }
                        });
                    }
                }
            }
        )
    }

    fn basic_mix(seed: u32, n: impl IntoIterator<Item = i64>) -> i64 {
        // very large RSA numbers
        const NUMS: [i64; 11] = [
            5867861265816020633, 1641017301357189737, 1879853796394333093, 7725969554000944399, 
            1876115753880194887, 5606298548251443349, 7362445722264101809, 1474238158419017591, 
            7014872933172552589, 7064322766213455167, 6453273212358484117,
        ];

        let mut nums_idx = seed as usize % NUMS.len();
        let iter = n.into_iter();
        let mut ret = (seed as i64).overflowing_mul(NUMS[nums_idx]).0.overflowing_add(NUMS[(nums_idx + 1) % NUMS.len()]).0;
        nums_idx = (nums_idx + 2) % NUMS.len();

        for n in iter {
            ret = ret.overflowing_mul(
                n.overflowing_mul(NUMS[nums_idx]).0.overflowing_add(NUMS[(nums_idx + 1) % NUMS.len()]).0
            ).0;

            nums_idx = (nums_idx + 2) % NUMS.len();
        }

        ret
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

        let mut chunk = self.chunk_from_height(height_sampler, chunk_pos);
        self.cave_transformer.apply(&mut chunk);
        chunk.blocks
    }

    fn chunk_from_height(&mut self, height_sampler: ChunkHeightSampler, chunk_pos: IVec2) -> TerrainChunk {
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

        TerrainChunk {
            height: height_array,
            blocks: Chunk {
                pos: chunk_pos,
                sections,
            }
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
}

pub struct TerrainChunk {
    pub height: Array2<u32>,
    pub blocks: Chunk,
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

#[cfg(test)]
mod test {
    use ndarray::arr2;

    use super::*;

    #[test]
    fn height_sampler() {
        let sampler = ChunkHeightSampler {
            height_data: arr2(&[[0.0, 0.0], [1.0, 1.0]])
        };

        assert_eq!(sampler.sample(Vec2::new(8.0, 4.9743)), 0.5);
    }
}