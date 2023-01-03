use bytemuck::{Zeroable, Pod};
use rgb::RGBA;
use ultraviolet::Vec3;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Zeroable, Pod)]
pub struct VertexRaw {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

vulkano::impl_vertex!(VertexRaw, position, color);

pub struct Vertex {
    pub position: Vec3,
    pub color: RGBA<f32>,
}

impl From<Vertex> for VertexRaw {
    fn from(value: Vertex) -> Self {
        Self {
            position: *value.position.as_array(),
            color: value.color.into(),
        }
    }
}