use bytemuck::{Zeroable, Pod};

#[repr(C)]
#[derive(Default, Copy, Clone, Zeroable, Pod)]
pub struct Vertex {
    position: [f32; 2],
}

vulkano::impl_vertex!(Vertex, position);