use crate::{render::{vertex::VertexRaw, texture::TextureAtlas}, world::block_data::StaticBlockData};

pub trait Renderable {
    fn get_vertices(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> Vec<VertexRaw>;
}

impl<T> Renderable for [T] 
where 
    T: Renderable
{
    fn get_vertices(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> Vec<VertexRaw> {
        self.iter().flat_map(|s| { s.get_vertices(atlas, block_data) }).collect()
    }
}