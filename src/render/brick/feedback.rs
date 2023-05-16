use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Feedback {
    pub top: u32,
    pub map_positions: [[i32; 2]; 32],
}

impl Feedback {
    pub fn empty() -> Self {
        Self::zeroed()
    }
}