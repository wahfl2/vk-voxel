use bytemuck::{Zeroable, Pod};

use crate::{render::texture::TextureAtlas, world::block_data::StaticBlockData};

/// A textured square of width 1 to be used with the shader storage buffer
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct BlockQuad {
    pub position: [f32; 3],
    pub face: u32,
    pub tex: [f32; 4],
}

impl BlockQuad {
    pub fn new(position: [f32; 3], tex: [f32; 4], face: u32) -> Self {
        Self { position, tex, face }
    }
}

pub trait ChunkRender {
    fn get_block_quads(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> Vec<BlockQuad>;
}

impl<T> ChunkRender for [T]
where 
    T: ChunkRender
{
    fn get_block_quads(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> Vec<BlockQuad> {
        self.iter().flat_map(|s| { s.get_block_quads(atlas, block_data) }).collect()
    }
}