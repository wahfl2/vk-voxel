use bytemuck::{Pod, Zeroable};
use ultraviolet::{Vec3, Vec2};

use crate::{util::util::{Sign, Facing}, render::{texture::{TextureAtlas, TextureHandle}}};

// Unused
pub struct RawQuad {
    pub points: [Vec3; 4],
}

/// A textured square of width 1 to be used with the shader storage buffer
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct BlockQuad {
    pub position: [f32; 3],
    pub face_tex: u32,
}

impl BlockQuad {
    pub fn new(position: [f32; 3], tex_index: u32, face: u32) -> Self {
        assert!(tex_index <= 0x1FFFFFF);
        assert!(face < 6);
        let face_tex = (face << 29) | tex_index;
        Self { position, face_tex }
    }
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

#[derive(Clone, Debug)]
pub struct QuadUV {
    pub min: Vec2,
    pub max: Vec2,
}

impl QuadUV {
    pub const fn tex_coords(&self) -> [Vec2; 4] {
        [
            self.max,
            Vec2::new(self.min.x, self.max.y),
            self.min,
            Vec2::new(self.max.x, self.min.y),
        ]
    }

    pub const fn to_raw(&self) -> [f32; 4] {
        [
            self.min.x,
            self.min.y,
            self.max.x,
            self.max.y,
        ]
    }
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

    pub fn into_block_quad(&self) -> BlockQuad {
        BlockQuad::new(
            self.center.into(),
            self.texture_handle.get_index(),
            self.facing.to_num() as u32,
        )
    }
}

// Unused
// impl Renderable for TexturedSquare {
//     fn get_vertices(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> Vec<VertexRaw> {
//         const INDICES: [usize; 6] = [
//             0, 1, 2,
//             0, 2, 3,
//         ];

//         let mut corners = match self.facing.axis {
//             Axis::X => Self::CORNER_OFFSETS_X,
//             Axis::Y => Self::CORNER_OFFSETS_Y,
//             Axis::Z => Self::CORNER_OFFSETS_Z,
//         }.map(|offset| { (self.center + offset).as_array().to_owned() });

//         if self.facing.sign == Sign::Negative {
//             corners.reverse();
//         }

//         let uv = atlas.get_uv(self.texture_handle);
//         let tex_coords = [
//             [uv.max.x, uv.max.y],
//             [uv.max.x, uv.min.y],
//             [uv.min.x, uv.min.y],
//             [uv.min.x, uv.max.y],
//         ];

//         INDICES.iter().map(|i| { 
//             VertexRaw {
//                 position: corners[*i],
//                 tex_coord: tex_coords[*i],
//             }
//          }).collect()
//     }
// }