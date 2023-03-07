use ultraviolet::Vec3;

use crate::{physics::solver::PhysicsSolver, render::camera::camera::Camera, event_handler::InputHandler};

use super::{components::{Player, Translation, Velocity, PhysicsEntity, Hitbox}, hierarchy::{Hierarchy, Parent}};

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
                gravity: 0.1,
                ..Default::default()
            }
        }
    }

    pub fn init_single_player(&mut self) {
        let player = Player::new("player");
        let translation = Translation(Vec3::new(0.0, -100.0, 0.0));
        let velocity = Velocity(Vec3::zero());
        let hitbox = Hitbox {
            half_extents: Vec3::new(0.9, 1.8, 0.9),
        };

        let player_entity = self.world.spawn((player, PhysicsEntity, translation, velocity, hitbox));

        let cam_offset = Translation(Vec3::new(0.0, 0.75, 0.0));
        let camera_entity = self.world.spawn((Camera::default(), cam_offset));
        
        self.world.add_child(player_entity, camera_entity);
        self.world.set_parent(camera_entity, player_entity);
    }

    pub fn tick(&mut self, input_handler: &InputHandler) {
        let binding = self.world.query_mut::<&mut Camera>();
        let (_, cam) = binding.into_iter().next().unwrap();

        let mut rot_delta = input_handler.mouse_delta * 0.004;
        rot_delta.x *= -1.0;

        cam.rotation += rot_delta.into();
        
        self.physics_solver.tick(&mut self.world);
    }

    pub fn get_camera(&self) -> Camera {
        let mut binding = self.world.query::<(&Camera, &Parent, &Translation)>();
        let (_, (cam, player, cam_translation)) = binding.into_iter().next().unwrap();

        let mut binding = self.world.query_one::<&Translation>(**player).unwrap();
        let player_translation = binding.get().unwrap();
        dbg!(player_translation);

        cam.with_pos(**player_translation + **cam_translation)
    }
}