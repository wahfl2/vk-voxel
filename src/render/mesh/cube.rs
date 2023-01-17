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
                Facing::RIGHT, self.texture_idx
            ),
            TexturedSquare::new(
                self.center - Vec3::unit_x() * HALF_SIZE, 
                Facing::LEFT, self.texture_idx
            ),
            TexturedSquare::new(
                self.center + Vec3::unit_y() * HALF_SIZE, 
                Facing::UP, self.texture_idx
            ),
            TexturedSquare::new(
                self.center - Vec3::unit_y() * HALF_SIZE, 
                Facing::DOWN, self.texture_idx
            ),
            TexturedSquare::new(
                self.center + Vec3::unit_z() * HALF_SIZE, 
                Facing::FORWARD, self.texture_idx
            ),
            TexturedSquare::new(
                self.center - Vec3::unit_z() * HALF_SIZE, 
                Facing::BACK, self.texture_idx
            ),
        ]
    }
}

impl Renderable for UnitCube {
    fn get_vertices(&self, atlas: &TextureAtlas) -> Vec<VertexRaw> {
        self.get_faces().get_vertices(atlas)
    }
}