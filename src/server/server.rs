use ultraviolet::Vec3;

use super::components::{Player, Translation, Velocity};

pub struct Server {
    pub world: hecs::World,
}

impl Server {
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
        }
    }

    pub fn init_single_player(&mut self) {
        let player = Player::new("player");
        let translation = Translation(Vec3::new(0.0, 50.0, 0.0));
        let velocity = Velocity(Vec3::zero());

        self.world.spawn((player, translation, velocity));
    }

    pub fn tick(&mut self) {
        
    }
}