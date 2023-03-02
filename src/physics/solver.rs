use hecs::World;
use ndarray::{Array3, Axis, s};
use ultraviolet::Vec3;

use crate::{util::{more_vec::UsizeVec3, util::{MoreCmp, VecRounding, VecAxisIndex}}, server::components::{PhysicsEntity, Translation, Velocity, Hitbox}};

pub struct PhysicsSolver {
    pub sub_steps: u32,
    pub gravity: f32,
    pub blocks: Option<PhysicsBlocks>,
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
        let rel_max = (entity_max - self.offset).ceil();

        fn get_index_for(v: Vec3) -> UsizeVec3 {
            UsizeVec3::new(v.x.max(0.0) as usize, v.y.max(0.0) as usize, v.z.max(0.0) as usize)
        }

        let min_index = get_index_for(rel_min);
        let max_index = get_index_for(rel_max);

        let offset = min_index.into_vec3();

        // Blocks relevant to the intersection test, everything else should be out of range
        let slice = self.blocks.slice(s![
            min_index.x..max_index.x, 
            min_index.y..max_index.y, 
            min_index.z..max_index.z
        ]);

        for ((x, y, z), b) in slice.indexed_iter() {
            if !b { continue; }

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
            blocks: None, 
            gravity: 0.0,
        }
    }
}

impl PhysicsSolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_blocks(&mut self, blocks: PhysicsBlocks) {
        self.blocks = Some(blocks);
    }

    pub fn tick(&mut self, world: &mut World) {
        let time_multiplier = 1.0 / self.sub_steps as f32;
        for _ in 0..self.sub_steps {
            self.sub_step(world, time_multiplier);
        }
    }

    fn sub_step(&mut self, world: &mut World, time_multiplier: f32) {
        let gravity = self.gravity * time_multiplier * time_multiplier;

        let q = world.query_mut::<(&PhysicsEntity, &mut Translation, &mut Velocity, &Hitbox)>();

        for (_, (_, pos, velocity, hitbox)) in q.into_iter() {
            **velocity += Vec3::new(0.0, gravity, 0.0);
            **velocity *= 0.99;

            match &self.blocks {
                None => **pos += **velocity,
                Some(blocks) => {
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

                    collide_axis(pos, velocity, hitbox, blocks, Axis(0));
                    collide_axis(pos, velocity, hitbox, blocks, Axis(2));
                    collide_axis(pos, velocity, hitbox, blocks, Axis(1));
                }
            }
        }
    }
}