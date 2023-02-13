use bytemuck::{Zeroable, Pod};
use ultraviolet::{Vec3, Vec2};
use vulkano::impl_vertex;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Zeroable, Pod)]
pub struct VertexRaw {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl_vertex!(VertexRaw, position, normal, tex_coord);

#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coords: Vec2,
}

impl From<Vertex> for VertexRaw {
    fn from(value: Vertex) -> Self {
        Self {
            position: value.position.into(),
            normal: value.normal.into(),
            tex_coord: value.tex_coords.into(),
        }
    }
}