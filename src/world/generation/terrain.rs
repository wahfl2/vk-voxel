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
use crate::util::util::{MoreCmp, VecRounding, MoreVecConstructors, AdditionalSwizzles};
use crate::world::block_data::Blocks;
use crate::world::chunk::{Chunk, CHUNK_HEIGHT};
use crate::world::section::{F_SECTION_SIZE, SECTION_SIZE, I_SECTION_SIZE};
use crate::world::{block_data::{StaticBlockData, BlockHandle}, section::Section};

use super::noise::{ScaleNoise2D, ScaleNoise3D};
use super::transformer::TerrainTransformer;

const CAVE_RADIUS: f32 = 4.0;

pub struct TerrainGenerator {
    pub planar_noise: ScaleNoise2D,
    pub world_noise: ScaleNoise3D,
    pub overall_height: ScaleNoise2D,
    pub seed: u32,
    cave_transformer: TerrainTransformer<Vec<Vec3>>,
    rng: Xoshiro128StarStar,
    cache: [BlockHandle; 4]
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

    fn cave_transformer(seed: u32) -> TerrainTransformer<Vec<Vec3>> {
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

                const CAVE_SAMPLES: usize = 500;

                let middle = Vec2::from(size) * (F_SECTION_SIZE.xz() * 0.5);
                let mut worm_pos = Vec3::new(middle.x, 20.0, middle.y);

                let min_accept = Vec2::splat(CAVE_RADIUS);
                let max_accept = Vec2::from(size * SECTION_SIZE.xz()) - min_accept;

                let mut ret = Vec::new();
                for i in 0..CAVE_SAMPLES {
                    let d = i as f64 * step;
                    let movement = MOVE_LENGTH * (MULTIPLIER * Vec3::new(
                        noise.get([d.x, filler.y, filler.z]) as f32,
                        noise.get([filler.x, d.y, filler.z]) as f32,
                        noise.get([filler.x, filler.y, d.z]) as f32,
                    )).normalized();

                    worm_pos += movement;
                    if worm_pos.xz().all_greater_than(&min_accept) && worm_pos.xz().all_less_than(&max_accept) {
                        ret.push(worm_pos);
                    }
                }

                ret
            },
            |chunk, offset, _size, data| {
                const RAD_SQ: f32 = CAVE_RADIUS * CAVE_RADIUS;
                let relative_pos = (chunk.blocks.pos - offset) * I_SECTION_SIZE.xz();

                let min = relative_pos;
                let max = relative_pos + (IVec2::one() * I_SECTION_SIZE.xz());

                let max_h = F_SECTION_SIZE.y * CHUNK_HEIGHT as f32;

                let min_chunk = Vec3::new(min.x as f32, 0.0, min.y as f32);
                let max_chunk = Vec3::new(max.x as f32, max_h, max.y as f32);

                for carve in data.iter() {
                    let min_carve = (*carve - Vec3::one() * (CAVE_RADIUS + 1.0)).clamped(Vec3::zero(), Vec3::one() * max_h).floor();
                    let max_carve = (*carve + Vec3::one() * (CAVE_RADIUS + 1.0)).clamped(Vec3::zero(), Vec3::one() * max_h).ceil();

                    if min_carve.any_greater_than(&max_chunk) || max_carve.any_less_than(&min_chunk) {
                        continue;
                    }

                    let min_section = ((min_carve.y / F_SECTION_SIZE.y).floor() as u32).min(CHUNK_HEIGHT - 1);
                    let max_section = ((max_carve.y / F_SECTION_SIZE.y).floor() as u32).min(CHUNK_HEIGHT - 1);

                    for i in min_section..=max_section {
                        let section = chunk.blocks.sections.get_mut(i as usize).unwrap();

                        let y_off = (i * SECTION_SIZE.y) as f32;
                        let mn = min_carve - Vec3::new(min_chunk.x, y_off, min_chunk.z);
                        let mx = max_carve - Vec3::new(min_chunk.x, y_off, min_chunk.z);

                        let min_section = UVec3::new(mn.x as u32, mn.y as u32, mn.z as u32).clamped(UVec3::zero(), SECTION_SIZE);
                        let max_section = UVec3::new(mx.x as u32, mx.y as u32, mx.z as u32).clamped(UVec3::zero(), SECTION_SIZE);

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

    // Should be replaced by just a noise function
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
            (chunk_pos * I_SECTION_SIZE.xz()).into(), 
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
        let height_array = Array2::from_shape_fn(
            (SECTION_SIZE.x as usize, SECTION_SIZE.z as usize), 
            |(x_step, y_step)| {
                let pos = off + Vec2::new(x_step as f32, y_step as f32);
                let height = (height_sampler.sample(pos) * 50.0 + 50.0).round() as u32;
                let low_gen = height.saturating_sub(4);
                if low_gen < lowest { lowest = low_gen; }
                if height > highest { highest = height; }

                height
            }
        );

        let section_low = lowest / SECTION_SIZE.y;
        let section_high = highest / SECTION_SIZE.y;

        let sections = Box::new(array::from_fn(|i| {
            let idx = i as u32;

            if idx < section_low {
                Section::full(self.cache[3])
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
        let height_offset = section_num * SECTION_SIZE.y;
        let mut ret = Section::empty();
        for (i, mut column) in ret.blocks.lanes_mut(Axis(1)).into_iter().enumerate() {
            let (x, z) = ((i / SECTION_SIZE.x as usize), (i % SECTION_SIZE.x as usize));
            let column_pos = (chunk_pos * I_SECTION_SIZE.xz()) + IVec2::new(x as i32, z as i32);
            let height = height_array[(x, z)];
            let relative_height = height.saturating_sub(height_offset);

            // All air
            if height < height_offset { continue; }

            // All stone
            if height > height_offset + SECTION_SIZE.y + 4 {
                column.fill(self.cache[3]);
                continue;
            }

            // let can_gen_grass = height >= height_offset.saturating_sub(1) && relative_height < SECTION_SIZE.y - 1;
            let stone_end = relative_height.saturating_sub(3).min(SECTION_SIZE.y - 1) as usize;
            let dirt_end = relative_height.min(SECTION_SIZE.y - 1) as usize;
            let mut c = [self.cache[0]; SECTION_SIZE.y as usize];

            c[0..stone_end].fill(self.cache[3]);
            c[stone_end..dirt_end].fill(self.cache[2]);
            c[relative_height.min(SECTION_SIZE.y - 1) as usize] = self.cache[1];

            // let grass_pos = IVec3::new(
            //     column_pos.x, 
            //     ((section_num * SECTION_SIZE.y) + relative_height + 1) as i32, 
            //     column_pos.y
            // );

            // if can_gen_grass && self.gen_grass_at(grass_pos) {
            //     c[relative_height as usize + 1] = self.cache[1];
            // }

            column.assign(&arr1(&c));
        }
        ret
    }

    fn gen_grass_at(&mut self, _pos: IVec3) -> bool {
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
        let step_size = F_SECTION_SIZE.xz() / res as f32;
        let arr = Array2::from_shape_fn((res + 1, res + 1), 
            |(step_x, step_y)| {
                let x = offset.x + (step_x as f32 * step_size.x);
                let y = offset.y + (step_y as f32 * step_size.y);
                noise.sample(Vec2::new(x, y), octaves) as f32
            }
        );

        Self {
            height_data: arr,
        }
    }

    /// Bi-linear interpolation of the input data
    pub fn sample(&self, relative_pos: Vec2) -> f32 {
        if relative_pos.any_greater_than(&F_SECTION_SIZE.xz()) || relative_pos.component_min() < 0.0 {
            panic!("Invalid sample position: {:?}", relative_pos);
        }

        let res = (self.height_data.len_of(Axis(0)) - 1) as f32;
        let mul = Vec2::splat(res) / F_SECTION_SIZE.xz();

        let rounded_x = (relative_pos.x * mul.x).floor();
        let rounded_y = (relative_pos.y * mul.y).floor();
        let data_x = rounded_x as usize;
        let data_y = rounded_y as usize;

        let recip = Vec2::one() / mul;
        let x0 = rounded_x * recip.x;
        let y0 = rounded_y * recip.y;
        let x1 = (rounded_x + 1.0) * recip.x;
        let y1 = (rounded_y + 1.0) * recip.y;

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