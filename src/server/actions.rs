use ultraviolet::{Vec3, Vec2};

pub enum PlayerAction {
    Movement(Vec3),
    Rotation(Vec2),
}