use ultraviolet::Vec3;

use crate::{render::texture::TextureHandle, util::util::Facing};

use super::quad::TexturedSquare;

#[derive(Debug, Clone)]
pub struct UnitCube {
    pub textures: [TextureHandle; 6],
}

impl UnitCube {
    pub fn new(textures: Vec<TextureHandle>) -> Option<Self> {
        Some(Self { textures: Self::expand_textures(textures)? })
    }

    pub fn get_faces(&self, offset: Vec3) -> [TexturedSquare; 6] {
        const HALF_SIZE: f32 = TexturedSquare::HALF_SIZE;

        [
            TexturedSquare::new(
                offset + Vec3::unit_x() * HALF_SIZE, 
                Facing::RIGHT, self.textures[0]
            ),
            TexturedSquare::new(
                offset - Vec3::unit_x() * HALF_SIZE, 
                Facing::LEFT, self.textures[1]
            ),
            TexturedSquare::new(
                offset + Vec3::unit_y() * HALF_SIZE, 
                Facing::UP, self.textures[2]
            ),
            TexturedSquare::new(
                offset - Vec3::unit_y() * HALF_SIZE, 
                Facing::DOWN, self.textures[3]
            ),
            TexturedSquare::new(
                offset + Vec3::unit_z() * HALF_SIZE, 
                Facing::FORWARD, self.textures[4]
            ),
            TexturedSquare::new(
                offset - Vec3::unit_z() * HALF_SIZE, 
                Facing::BACK, self.textures[5]
            ),
        ]
    }

    fn expand_textures(textures: Vec<TextureHandle>) -> Option<[TextureHandle; 6]> {
        match textures.len() {
            0 => panic!("No textures"),
            1 => Some([textures[0]; 6]),
            3 => {
                let t = textures.clone();
                Some([t[1], t[1], t[0], t[2], t[1], t[1]])
            },
            6 => Some(textures[..6].try_into().unwrap()),
            len => panic!("Uninferrable texture amount: {len}\nPrefer expanding it to 6 textures."),
        }
    }
}

// impl Renderable for UnitCube {
//     fn get_vertices(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> Vec<VertexRaw> {
//         self.get_faces().get_vertices(atlas, block_data)
//     }
// }