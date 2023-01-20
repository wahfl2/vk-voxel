use ultraviolet::Vec3;

use crate::{render::{texture::{TextureAtlas, TextureHandle}, vertex::VertexRaw}, util::Facing};

use super::{renderable::Renderable, quad::TexturedSquare};

#[derive(Debug, Clone)]
pub struct UnitCube {
    pub center: Vec3,
    pub textures: Vec<TextureHandle>,
}

impl UnitCube {
    pub fn new(center: Vec3, textures: Vec<TextureHandle>) -> Self {
        Self { center, textures }
    }

    pub fn from_textures(textures: Vec<TextureHandle>) -> Self {
        Self::new(Vec3::zero(), textures)
    }

    pub fn get_faces(&self) -> [TexturedSquare; 6] {
        const HALF_SIZE: f32 = TexturedSquare::HALF_SIZE;

        let face_textures = match self.textures.len() {
            0 => panic!("No textures"),
            1 => [self.textures[0]; 6],
            3 => {
                let t = self.textures.clone();
                [t[1], t[1], t[0], t[2], t[1], t[1]]
            },
            6 => self.textures[..6].try_into().unwrap(),
            len => panic!("Uninferrable texture amount: {len}\nPrefer expanding it to 6 textures."),
        };

        [
            TexturedSquare::new(
                self.center + Vec3::unit_x() * HALF_SIZE, 
                Facing::RIGHT, face_textures[0]
            ),
            TexturedSquare::new(
                self.center - Vec3::unit_x() * HALF_SIZE, 
                Facing::LEFT, face_textures[1]
            ),
            TexturedSquare::new(
                self.center + Vec3::unit_y() * HALF_SIZE, 
                Facing::UP, face_textures[2]
            ),
            TexturedSquare::new(
                self.center - Vec3::unit_y() * HALF_SIZE, 
                Facing::DOWN, face_textures[3]
            ),
            TexturedSquare::new(
                self.center + Vec3::unit_z() * HALF_SIZE, 
                Facing::FORWARD, face_textures[4]
            ),
            TexturedSquare::new(
                self.center - Vec3::unit_z() * HALF_SIZE, 
                Facing::BACK, face_textures[5]
            ),
        ]
    }
}

impl Renderable for UnitCube {
    fn get_vertices(&self, atlas: &TextureAtlas) -> Vec<VertexRaw> {
        self.get_faces().get_vertices(atlas)
    }
}