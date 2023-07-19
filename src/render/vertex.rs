use bytemuck::{Zeroable, Pod};
use ultraviolet::{Vec3, Vec2};
use vulkano::pipeline::graphics::vertex_input::Vertex as VertDerive;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Zeroable, Pod, VertDerive)]
pub struct VertexRaw {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub normal: [f32; 3],
    #[format(R32G32_SFLOAT)]
    pub tex_coord: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Zeroable, Pod, VertDerive)]
pub struct Vertex2D {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],
}

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