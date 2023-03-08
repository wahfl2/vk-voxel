use hecs::World;
use ndarray::{Array3, Axis, s};
use ultraviolet::{Vec3, IVec2, IVec3};

use crate::{util::{more_vec::UsizeVec3, util::{MoreCmp, VecRounding, VecAxisIndex, Vec3Trunc, AdditionalSwizzles}}, server::components::{PhysicsEntity, Translation, Velocity, Hitbox}, world::{world::WorldBlocks, block_data::{StaticBlockData, BlockType, BlockHandle}}};

pub struct PhysicsSolver {
    pub sub_steps: u32,
    pub gravity: f32,
}

pub struct PhysicsBlocks {
    pub offset: Vec3,
    pub blocks: Array3<bool>,
}

impl PhysicsBlocks {
    pub fn intersection_test(&self, pos: Vec3, hitbox: &Hitbox, axis: Axis) -> Option<f32> {
        let blocks_size = UsizeVec3::new(
            self.blocks.len_of(Axis(0)),
            self.blocks.len_of(Axis(1)),
            self.blocks.len_of(Axis(2)),
        ).into_vec3();

        let blocks_min = self.offset;
        let blocks_max = blocks_min + blocks_size;

        let entity_min = pos - hitbox.half_extents;
        let entity_max = pos + hitbox.half_extents;

        if !entity_max.all_greater_than(&blocks_min) || !entity_min.all_less_than(&blocks_max) {
            return None
        }

        let rel_min = (entity_min - self.offset).floor();
        let rel_max = (entity_max - self.offset).floor();

        fn get_index_for(v: Vec3) -> UsizeVec3 {
            UsizeVec3::new(v.x.max(0.0) as usize, v.y.max(0.0) as usize, v.z.max(0.0) as usize)
        }

        let min_index = get_index_for(rel_min);
        let offset = min_index.into_vec3();

        for ((x, y, z), b) in self.blocks.indexed_iter() {
            if !b { continue; }

            println!("relevant true block!!");

            let block_min = UsizeVec3::new(x, y, z).into_vec3() + offset;
            let block_max = block_min + Vec3::one();

            let grtr = rel_max.all_greater_than(&block_min);
            let less = rel_min.all_less_than(&block_max);

            if grtr && less {
                if grtr && rel_max.all_less_than(&block_max) {
                    return Some(block_min.get(axis) - rel_max.get(axis))
                } else {
                    return Some(block_max.get(axis) - rel_min.get(axis))
                }
            }
        }

        None
    }
}

impl Default for PhysicsSolver {
    fn default() -> Self {
        Self {
            sub_steps: 0,
            gravity: 0.0,
        }
    }
}

impl PhysicsSolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self, world: &mut World, world_blocks: &WorldBlocks, block_data: &StaticBlockData) {
        let time_multiplier = 1.0 / self.sub_steps as f32;
        for _ in 0..self.sub_steps {
            self.sub_step(world, time_multiplier, world_blocks, block_data);
        }
    }

    fn sub_step(&mut self, world: &mut World, time_multiplier: f32, blocks: &WorldBlocks, block_data: &StaticBlockData) {
        let gravity = self.gravity * time_multiplier * time_multiplier;

        let q = world.query_mut::<(&PhysicsEntity, &mut Translation, &mut Velocity, &Hitbox)>();

        for (_, (_, pos, velocity, hitbox)) in q.into_iter() {
            **velocity += Vec3::new(0.0, gravity, 0.0);
            **velocity *= 0.99;

            if blocks.loaded_chunks.is_empty() {
                **pos += **velocity
            } else {
                let min = (**pos - hitbox.half_extents).floor().into_i();
                let max = (**pos + hitbox.half_extents).floor().into_i();

                let s = max - min;
                let mut arr = Array3::from_elem(
                    (s.x as usize, s.y as usize, s.z as usize), 
                    false
                );

                let min_section = (min / 16).max_by_component(IVec3::new(i32::MIN, 0, i32::MIN));
                let max_section = (max / 16).min_by_component(IVec3::new(i32::MAX, 255, i32::MAX));

                if min_section.y == max_section.y && (min_section.y == 0 || max_section.y == 255) {
                    continue;
                }

                for chunk_x in min_section.x..max_section.x {
                    for chunk_z in min_section.z..max_section.z {
                        let chunk_pos = IVec2::new(chunk_x, chunk_z);
                        let chunk_offset = chunk_pos * 16;

                        let chunk = blocks.loaded_chunks.get(&chunk_pos);
                        if let None = chunk { continue; }
                        let chunk = chunk.unwrap();

                        for section_i in min_section.y..max_section.y {
                            let section = &chunk.sections[section_i as usize];
                            let section_offset = IVec3::new(chunk_offset.x, section_i * 16, chunk_offset.y);

                            let relative_min = min - section_offset;
                            let relative_max = max - section_offset;

                            let min_sec_index = relative_min.clamped(IVec3::zero(), 15 * IVec3::one());
                            let max_sec_index = relative_max.clamped(IVec3::zero(), 15 * IVec3::one());

                            let min_arr_index = min_sec_index - relative_min;
                            let max_arr_index = max_sec_index - relative_min;

                            arr.slice_mut(s![
                                min_arr_index.x..max_arr_index.x,
                                min_arr_index.y..max_arr_index.y,
                                min_arr_index.z..max_arr_index.z,
                            ]).assign(
                                &section.blocks.slice(s![
                                    min_sec_index.x..max_sec_index.x,
                                    min_sec_index.y..max_sec_index.y,
                                    min_sec_index.z..max_sec_index.z,
                                ]).map(|b| {
                                    block_data.get(b).block_type != BlockType::None
                                })
                            );                            
                        }
                    }
                }

                let blocks = PhysicsBlocks {
                    offset: min.into(),
                    blocks: arr,
                };

                fn collide_axis(pos: &mut Vec3, velocity: &mut Vec3, hitbox: &Hitbox, blocks: &PhysicsBlocks, axis: Axis) {
                    // Move the entity in the specified direction
                    *pos.get_mut(axis) += velocity.get(axis);
                    // Test for intersection and move back the collision distance
                    let intersection = blocks.intersection_test(*pos, hitbox, axis);

                    if let Some(dist) = intersection {
                        *pos.get_mut(axis) += dist;
                        velocity.set(axis, 0.0);
                    }
                }

                collide_axis(pos, velocity, hitbox, &blocks, Axis(0));
                collide_axis(pos, velocity, hitbox, &blocks, Axis(2));
                collide_axis(pos, velocity, hitbox, &blocks, Axis(1));
            }
        }
    }
}