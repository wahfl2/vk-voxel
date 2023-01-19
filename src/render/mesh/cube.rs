use ultraviolet::Vec3;

use crate::{render::{texture::TextureAtlas, vertex::VertexRaw}, util::Facing};

use super::{renderable::Renderable, quad::TexturedSquare};

pub struct UnitCube {
    pub center: Vec3,
    pub texture_idx: usize,
}

impl UnitCube {
    pub fn get_faces(&self) -> [TexturedSquare; 6] {
        const HALF_SIZE: f32 = TexturedSquare::HALF_SIZE;

        [
            TexturedSquare::new(
                self.center + Vec3::unit_x() * HALF_SIZE, 
                Facing::RIGHT, 2
            ),
            TexturedSquare::new(
                self.center - Vec3::unit_x() * HALF_SIZE, 
                Facing::LEFT, 2
            ),
            TexturedSquare::new(
                self.center + Vec3::unit_y() * HALF_SIZE, 
                Facing::UP, 3
            ),
            TexturedSquare::new(
                self.center - Vec3::unit_y() * HALF_SIZE, 
                Facing::DOWN, 1
            ),
            TexturedSquare::new(
                self.center + Vec3::unit_z() * HALF_SIZE, 
                Facing::FORWARD, 2
            ),
            TexturedSquare::new(
                self.center - Vec3::unit_z() * HALF_SIZE, 
                Facing::BACK, 2
            ),
        ]
    }
}

impl Renderable for UnitCube {
    fn get_vertices(&self, atlas: &TextureAtlas) -> Vec<VertexRaw> {
        self.get_faces().get_vertices(atlas)
    }
}