use hecs::World;
use ndarray::{Array3, Axis, s, AssignElem};
use ultraviolet::{Vec3, IVec2, IVec3};

use crate::{util::{more_vec::UsizeVec3, util::{MoreCmp, VecRounding, VecAxisIndex, Vec3Trunc, AdditionalSwizzles, MoreVecOps}}, server::components::{PhysicsEntity, Translation, Velocity, Hitbox}, world::{world::WorldBlocks, block_data::{StaticBlockData, BlockType, BlockHandle}}};

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

        let rel_min = entity_min - self.offset;
        let rel_max = entity_max - self.offset;

        for ((x, y, z), b) in self.blocks.indexed_iter() {
            if !b { continue; }

            let block_min = UsizeVec3::new(x, y, z).into_vec3() - (Vec3::one() * 0.5);
            let block_max = block_min + Vec3::one();

            let grtr = rel_max.all_greater_than(&block_min);
            let less = rel_min.all_less_than(&block_max);

            if grtr && less {
                let dist_down = block_min.get(axis) - rel_max.get(axis);
                let dist_up   = block_max.get(axis) - rel_min.get(axis);

                return Some(
                    if dist_down.abs() > dist_up.abs() {
                        dist_up
                    } else {
                        dist_down
                    }
                )
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

    pub fn tick(&mut self, delta_time: f32, world: &mut World, world_blocks: &WorldBlocks, block_data: &StaticBlockData) {
        let time_multiplier = delta_time / self.sub_steps as f32;
        for _ in 0..self.sub_steps {
            self.sub_step(world, time_multiplier, world_blocks, block_data);
        }
    }

    fn sub_step(&mut self, world: &mut World, time_multiplier: f32, blocks: &WorldBlocks, block_data: &StaticBlockData) {

        let q = world.query_mut::<(&PhysicsEntity, &mut Translation, &mut Velocity, &Hitbox)>();

        for (_, (_, pos, velocity, hitbox)) in q.into_iter() {
            **velocity += Vec3::new(0.0, self.gravity * time_multiplier.sqrt(), 0.0);

            const DAMPING: Vec3 = Vec3::new(0.9, 0.1, 0.9);
            **velocity *= (Vec3::one() - DAMPING).powf(time_multiplier);

            if blocks.loaded_chunks.is_empty() {
                **pos += **velocity * time_multiplier
            } else {
                let real_pos = **pos;
                let min = (real_pos - hitbox.half_extents).floor().into_i() - (3 * IVec3::one());
                let max = (real_pos + hitbox.half_extents).ceil().into_i()  + (3 * IVec3::one());

                let s = max - min;
                let mut arr = Array3::from_elem(
                    (s.x as usize, s.y as usize, s.z as usize), 
                    true
                );

                let mut check_visit_arr = Array3::from_elem(
                    (s.x as usize, s.y as usize, s.z as usize), 
                    0
                );

                const SIXTEENTH: f32 = 1.0 / 16.0;

                let min_section = ((Vec3::from(min) * SIXTEENTH).floor().into_i()).clamped(
                    IVec3::new(i32::MIN, 0,   i32::MIN),
                    IVec3::new(i32::MAX, 255, i32::MAX)
                );

                let max_section = ((Vec3::from(max) * SIXTEENTH).floor().into_i()).clamped(
                    IVec3::new(i32::MIN, 0,   i32::MIN),
                    IVec3::new(i32::MAX, 255, i32::MAX)
                );

                if min_section.y != max_section.y || (min_section.y != 0 && max_section.y != 255) {
                    for chunk_x in min_section.x..=max_section.x {
                        for chunk_z in min_section.z..=max_section.z {
                            let chunk_pos = IVec2::new(chunk_x, chunk_z);
                            let chunk_offset = chunk_pos * 16;
    
                            let chunk = blocks.loaded_chunks.get(&chunk_pos);
                            if let None = chunk { continue; }
                            let chunk = chunk.unwrap();
    
                            for section_i in min_section.y..=max_section.y {
                                let section = &chunk.sections[section_i as usize];
                                let section_offset = IVec3::new(chunk_offset.x, section_i * 16, chunk_offset.y);
    
                                let relative_min = min - section_offset;
                                let relative_max = max - section_offset;
    
                                let min_sec_index = relative_min.clamped(IVec3::zero(), 16 * IVec3::one());
                                let max_sec_index = relative_max.clamped(IVec3::zero(), 16 * IVec3::one());
    
                                let min_arr_index = min_sec_index - relative_min;
                                let max_arr_index = max_sec_index - relative_min;

                                if (max_arr_index - min_arr_index).component_min() < 1 { continue; }

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
                                        block_data.get(b).block_type == BlockType::Full
                                    })
                                );

                                check_visit_arr.slice_mut(s![
                                    min_arr_index.x..max_arr_index.x,
                                    min_arr_index.y..max_arr_index.y,
                                    min_arr_index.z..max_arr_index.z,
                                ]).mapv_inplace(|i| { i+1 });
                            }
                        }
                    }
                }

                let blocks = PhysicsBlocks {
                    offset: min.into(),
                    blocks: arr,
                };

                // This check can probably be removed
                for i in check_visit_arr.into_iter() {
                    if i == 0 {
                        println!("Not all blocks visited!\nMin: {:?}, Max: {:?}", min, max);
                        break;
                    } else if i > 1 {
                        println!("Block(s) visited twice!\nMin: {:?}, Max: {:?}", min, max);
                        break;
                    }
                }

                fn collide_axis(time_multiplier: f32, pos: &mut Vec3, velocity: &mut Vec3, hitbox: &Hitbox, blocks: &PhysicsBlocks, axis: Axis) {
                    // Move the entity in the specified direction
                    *pos.get_mut(axis) += velocity.get(axis) * time_multiplier;
                    // Test for intersection and move back the collision distance
                    let intersection = blocks.intersection_test(*pos, hitbox, axis);

                    if let Some(dist) = intersection {
                        *pos.get_mut(axis) += dist;
                        velocity.set(axis, 0.0);
                    }
                }

                collide_axis(time_multiplier, pos, velocity, hitbox, &blocks, Axis(0));
                collide_axis(time_multiplier, pos, velocity, hitbox, &blocks, Axis(2));
                collide_axis(time_multiplier, pos, velocity, hitbox, &blocks, Axis(1));
            }
        }
    }
}

#[test]
fn int_div_test() {
    println!(" 1 / 16 = {}", 1 / 16);
    println!("-1 / 16 = {}", -1 / 16);


}