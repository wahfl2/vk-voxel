use bytemuck::Zeroable;
use ultraviolet::Vec3;
use vulkano::buffer::BufferContents;

use crate::{util::util::{VecRounding, UVecTrunc, IVecTrunc, VecModPos}, render::brick::brickgrid::BRICKGRID_SIZE};

pub struct Surfel {
    pub accumulated: Vec3,
    pub radius: f32,
    pub normal: Vec3,
    pub world_pos: Vec3,
}

impl Surfel {
    pub fn new(accumulated: Vec3, radius: f32, normal: Vec3, world_pos: Vec3) -> Self {
        Self { accumulated, radius, normal, world_pos }
    }

    pub fn to_raw(&self) -> SurfelRaw {
        if self.normal.mag_sq() > 1.0 {
            panic!("Normal big!");
        }

        const RECIP_16: f32 = 1.0 / 16.0;
        let chunk_pos = (self.world_pos * RECIP_16).floor();
        let chunk_offset = (self.world_pos - chunk_pos).floor();
        let offset = (self.world_pos - chunk_pos) - chunk_offset;

        let chunk_pos = chunk_pos.into_i().mod_pos(BRICKGRID_SIZE.into());
        let chunk_offset = chunk_offset.into_u();
        let offset = (offset * 65536.0).floor().into_u();
        let normal = (self.normal * 65536.0).floor().into_u();

        let packed_chunk_pos = (chunk_pos.x << 15) | (chunk_pos.y << 10) | (chunk_pos.z);
        let packed_chunk_offset = (chunk_offset.x << 6) | (chunk_offset.y << 3) | (chunk_offset.z);

        SurfelRaw {
            accumulated: self.accumulated.into(),
            radius: self.radius,
            normal: normal.as_array().map(|n| n as u16),
            offset: offset.as_array().map(|n| n as u16),
            packed_chunk_offset,
            packed_chunk_pos,
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, BufferContents, Zeroable)]
pub struct SurfelRaw {
    pub accumulated: [f32; 3],
    pub radius: f32,
    pub normal: [u16; 3],
    pub offset: [u16; 3],

    // empty (23) chunk offset (3 3 3)
    pub packed_chunk_offset: u32,
    // empty (7) chunk pos (10 5 10)
    pub packed_chunk_pos: u32,
}

impl Default for SurfelRaw {
    fn default() -> Self {
        Self {
            radius: f32::NAN,
            ..Self::zeroed()
        }
    }
}