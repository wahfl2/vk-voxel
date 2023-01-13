use bytemuck::{Zeroable, Pod};
use ultraviolet::{Vec3, Vec2};

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Zeroable, Pod)]
pub struct VertexRaw {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

vulkano::impl_vertex!(VertexRaw, position, tex_coords);

pub struct Vertex {
    pub position: Vec3,
    pub tex_coords: Vec2,
}

impl From<Vertex> for VertexRaw {
    fn from(value: Vertex) -> Self {
        Self {
            position: value.position.into(),
            tex_coords: value.tex_coords.into(),
        }
    }
}