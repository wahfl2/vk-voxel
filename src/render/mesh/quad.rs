use bytemuck::{Pod, Zeroable};
use ultraviolet::{Vec3, Vec2};

use crate::{util::util::{Axis, Sign, Facing}, render::{vertex::VertexRaw, texture::{TextureAtlas, TextureHandle}}, world::block_data::StaticBlockData};

use super::{renderable::Renderable, chunk_render::{ChunkRender, BlockQuad}};

// Unused
pub struct RawQuad {
    pub points: [Vec3; 4],
}

pub struct AxisAlignedQuad {
    pub plane: f32,
    pub min: Vec2,
    pub max: Vec2,
    pub facing: Facing,
}

impl AxisAlignedQuad {
    pub fn get_corners(&self) -> [Vec3; 4] {
        let mut ret = [
            self.facing.axis.point_on_plane(self.plane, self.max),
            self.facing.axis.point_on_plane(self.plane, Vec2::new(self.min.x, self.max.y)),
            self.facing.axis.point_on_plane(self.plane, self.min),
            self.facing.axis.point_on_plane(self.plane, Vec2::new(self.max.x, self.min.y))
        ];

        if self.facing.sign == Sign::Negative { ret.reverse(); }
        ret
    }
}

pub struct QuadUV {
    pub min: Vec2,
    pub max: Vec2,
}

/// A textured quad with a width and length of 1.0
#[derive(Debug, Clone)]
pub struct TexturedSquare {
    pub center: Vec3,
    pub facing: Facing,
    pub texture_handle: TextureHandle,
}

impl TexturedSquare {
    /// The length and width of all textured squares
    pub const SIZE: f32 = 1.0;
    pub const HALF_SIZE: f32 = Self::SIZE / 2.0;

    pub const CORNER_OFFSETS_X: [Vec3; 4] = [
        Vec3::new(0.0, -Self::HALF_SIZE, -Self::HALF_SIZE),
        Vec3::new(0.0, Self::HALF_SIZE, -Self::HALF_SIZE),
        Vec3::new(0.0, Self::HALF_SIZE, Self::HALF_SIZE),
        Vec3::new(0.0, -Self::HALF_SIZE, Self::HALF_SIZE),
    ];

    pub const CORNER_OFFSETS_Y: [Vec3; 4] = [
        Vec3::new(Self::HALF_SIZE, 0.0, Self::HALF_SIZE),
        Vec3::new(Self::HALF_SIZE, 0.0, -Self::HALF_SIZE),
        Vec3::new(-Self::HALF_SIZE, 0.0, -Self::HALF_SIZE),
        Vec3::new(-Self::HALF_SIZE, 0.0, Self::HALF_SIZE),
    ];

    pub const CORNER_OFFSETS_Z: [Vec3; 4] = [
        Vec3::new(Self::HALF_SIZE, -Self::HALF_SIZE, 0.0),
        Vec3::new(Self::HALF_SIZE, Self::HALF_SIZE, 0.0),
        Vec3::new(-Self::HALF_SIZE, Self::HALF_SIZE, 0.0),
        Vec3::new(-Self::HALF_SIZE, -Self::HALF_SIZE, 0.0),
    ];

    pub fn new(center: Vec3, facing: Facing, texture_handle: TextureHandle) -> Self {
        Self { center, facing, texture_handle }
    }

    pub fn into_block_quad(&self, atlas: &TextureAtlas) -> BlockQuad {
        let uv = atlas.get_uv(self.texture_handle);
        BlockQuad::new(
            self.center.into(),
            [uv.min.x, uv.min.y, uv.max.x, uv.max.y],
            self.facing.to_num() as u32,
        )
    }
}

// Unused
impl Renderable for TexturedSquare {
    fn get_vertices(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> Vec<VertexRaw> {
        const INDICES: [usize; 6] = [
            0, 1, 2,
            0, 2, 3,
        ];

        let mut corners = match self.facing.axis {
            Axis::X => Self::CORNER_OFFSETS_X,
            Axis::Y => Self::CORNER_OFFSETS_Y,
            Axis::Z => Self::CORNER_OFFSETS_Z,
        }.map(|offset| { (self.center + offset).as_array().to_owned() });

        if self.facing.sign == Sign::Negative {
            corners.reverse();
        }

        let uv = atlas.get_uv(self.texture_handle);
        let tex_coords = [
            [uv.max.x, uv.max.y],
            [uv.max.x, uv.min.y],
            [uv.min.x, uv.min.y],
            [uv.min.x, uv.max.y],
        ];

        INDICES.iter().map(|i| { 
            VertexRaw {
                position: corners[*i],
                tex_coords: tex_coords[*i],
            }
         }).collect()
    }
}