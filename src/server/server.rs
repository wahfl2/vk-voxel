use ultraviolet::Vec3;

use crate::physics::solver::PhysicsSolver;

use super::components::{Player, Translation, Velocity, PhysicsEntity};

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
                gravity: -0.5,
                ..Default::default()
            }
        }
    }

    pub fn init_single_player(&mut self) {
        let player = Player::new("player");
        let translation = Translation(Vec3::new(0.0, 50.0, 0.0));
        let velocity = Velocity(Vec3::zero());

        self.world.spawn((player, PhysicsEntity, translation, velocity));
    }

    pub fn tick(&mut self) {
        self.physics_solver.tick(&mut self.world);
    }
}