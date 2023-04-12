use derive_more::{Deref, DerefMut};
use ultraviolet::Vec3;

use super::actions::PlayerAction;

pub struct Player {
    pub username: String,
    pub actions: Vec<PlayerAction>,
}

impl Player {
    pub fn new(username: &str) -> Self { 
        Self { 
            username: username.to_string(),
            actions: Vec::new(),
        } 
    }
}

#[derive(Debug, Deref, DerefMut)]
pub struct Translation(pub Vec3);

#[derive(Debug, Deref, DerefMut)]
pub struct Velocity(pub Vec3);

#[derive(Clone, Copy)]
pub struct Hitbox {
    pub half_extents: Vec3,
}

pub struct PhysicsEntity;

pub struct Gravity;