use std::f32::consts::PI;

use ultraviolet::{Vec3, Vec2, Rotor2};
use winit::event::VirtualKeyCode;

use crate::{physics::solver::PhysicsSolver, render::camera::camera::Camera, event_handler::InputHandler, world::{world_blocks::WorldBlocks, block_data::StaticBlockData}};

use super::{components::{Player, Translation, Velocity, PhysicsEntity, Hitbox, Gravity}, hierarchy::{Hierarchy, Parent}};

pub struct Server {
    pub world: hecs::World,
    pub physics_solver: PhysicsSolver,
}

impl Server {
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
            physics_solver: PhysicsSolver {
                sub_steps: 4,
                gravity: -1.5,
                ..Default::default()
            }
        }
    }

    pub fn init_single_player(&mut self) {
        let player = Player::new("player");
        let translation = Translation(Vec3::new(0.0, 100.0, 0.0));
        let velocity = Velocity(Vec3::zero());
        let hitbox = Hitbox {
            half_extents: Vec3::new(0.3, 0.9, 0.3),
        };

        let player_entity = self.world.spawn((
            player, 
            PhysicsEntity, 
            // Gravity, 
            translation, 
            velocity, 
            hitbox
        ));

        let cam_offset = Translation(Vec3::new(0.0, 0.72, 0.0));
        let camera_entity = self.world.spawn((Camera::default(), cam_offset));
        
        self.world.add_child(player_entity, camera_entity);
        self.world.set_parent(camera_entity, player_entity);
    }

    pub fn tick(&mut self, delta_time: f32, input_handler: &InputHandler, world_blocks: &WorldBlocks, block_data: &StaticBlockData) {
        let binding = self.world.query_mut::<&mut Camera>();
        let (_, cam) = binding.into_iter().next().unwrap();

        let mut rot_delta = input_handler.mouse_delta * 0.004;
        rot_delta.x *= -1.0;

        cam.rotation += rot_delta.into();

        const HALF_PI: f32 = PI / 2.0;
        cam.rotation.pitch = cam.rotation.pitch.clamp(-HALF_PI, HALF_PI);
        let cam_yaw = cam.rotation.yaw;

        let binding = self.world.query_mut::<(&Player, Option<&Gravity>, &mut Velocity)>();
        let (_, (_, gravity, vel)) = binding.into_iter().next().unwrap();

        const MOVEMENT_SPEED: f32 = 2.0;

        // represents movement on the xz plane
        let mut movement = Vec2::zero();
        if input_handler.is_pressed(VirtualKeyCode::W) {
            movement.y -= 1.0;
        }
        if input_handler.is_pressed(VirtualKeyCode::A) {
            movement.x -= 1.0;
        }
        if input_handler.is_pressed(VirtualKeyCode::S) {
            movement.y += 1.0;
        }
        if input_handler.is_pressed(VirtualKeyCode::D) {
            movement.x += 1.0;
        }

        if movement != Vec2::zero() {
            movement = movement.normalized() * MOVEMENT_SPEED;
            movement.rotate_by(Rotor2::from_angle(-cam_yaw));
        }

        // incredible input handling i know
        if input_handler.is_pressed(VirtualKeyCode::Space) {
            if gravity.is_none() {
                vel.y += MOVEMENT_SPEED;
            } else {
                vel.y = 6.0;
            }
        }

        if gravity.is_none() && input_handler.is_pressed(VirtualKeyCode::LShift) {
            vel.y -= MOVEMENT_SPEED;
        }

        **vel += Vec3::new(movement.x, 0.0, movement.y);
        
        self.physics_solver.tick(delta_time, &mut self.world, world_blocks, block_data);
    }

    pub fn get_camera(&self) -> Camera {
        let mut binding = self.world.query::<(&Camera, &Parent, &Translation)>();
        let (_, (cam, player, cam_translation)) = binding.into_iter().next().unwrap();

        let mut binding = self.world.query_one::<&Translation>(**player).unwrap();
        let player_translation = binding.get().unwrap();

        cam.with_pos(**player_translation + **cam_translation)
    }
}