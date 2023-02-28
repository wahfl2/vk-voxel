use ndarray::{Array3, Axis, s};
use ultraviolet::Vec3;

use crate::util::{more_vec::UsizeVec3, util::{MoreCmp, VecRounding, VecAxisIndex}};

pub struct PhysicsSolver {
    pub sub_steps: u32,
    pub gravity: f32,
    pub entities: Vec<PhysicsEntity>,
    pub blocks: Option<PhysicsBlocks>,
}

#[derive(Debug, Clone)]
pub struct PhysicsEntity {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub half_extents: Vec3,
}

impl PhysicsEntity {
    pub fn new(pos: Vec3, velocity: Vec3, half_extents: Vec3) -> Self {
        Self { pos, velocity, half_extents }
    }

    pub fn new_still(pos: Vec3, half_extents: Vec3) -> Self {
        Self::new(pos, Vec3::zero(), half_extents)
    }

    fn update_velocity(&mut self, acceleration: Vec3) {
        self.velocity += acceleration;
        self.velocity *= 0.99;
    }
}

pub struct PhysicsBlocks {
    pub offset: Vec3,
    pub blocks: Array3<bool>,
}

impl PhysicsBlocks {
    pub fn intersection_test(&self, entity: &PhysicsEntity, axis: Axis) -> Option<f32> {
        let blocks_size = UsizeVec3::new(
            self.blocks.len_of(Axis(0)),
            self.blocks.len_of(Axis(1)),
            self.blocks.len_of(Axis(2)),
        ).into_vec3();

        let blocks_min = self.offset;
        let blocks_max = blocks_min + blocks_size;

        let entity_min = entity.pos - entity.half_extents;
        let entity_max = entity.pos + entity.half_extents;

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
            entities: Vec::new(), 
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

    pub fn tick(&mut self) {
        let time_multiplier = 1.0 / self.sub_steps as f32;
        for _ in 0..self.sub_steps {
            self.sub_step(time_multiplier);
        }
    }

    fn sub_step(&mut self, time_multiplier: f32) {
        let gravity = self.gravity * time_multiplier * time_multiplier;
        for entity in self.entities.iter_mut() {
            entity.update_velocity(Vec3::new(0.0, gravity, 0.0));

            match &self.blocks {
                None => entity.pos += entity.velocity,
                Some(blocks) => {
                    fn collide_axis(entity: &mut PhysicsEntity, blocks: &PhysicsBlocks, axis: Axis) {
                        *entity.pos.get_mut(axis) += entity.velocity.get(axis);
                        let intersection = blocks.intersection_test(entity, axis);

                        if let Some(dist) = intersection {
                            *entity.pos.get_mut(axis) += dist;
                            entity.velocity.set(axis, 0.0);
                        }
                    }

                    collide_axis(entity, blocks, Axis(0));
                    collide_axis(entity, blocks, Axis(2));
                    collide_axis(entity, blocks, Axis(1));
                }
            }
        }
    }
}