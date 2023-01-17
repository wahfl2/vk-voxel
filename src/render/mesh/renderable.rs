use crate::{render::{vertex::VertexRaw, texture::TextureAtlas}};

pub trait Renderable {
    fn get_vertices(&self, atlas: &TextureAtlas) -> Vec<VertexRaw>;
}

impl<T> Renderable for [T] 
where 
    T: Renderable
{
    fn get_vertices(&self, atlas: &TextureAtlas) -> Vec<VertexRaw> {
        self.iter().flat_map(|s| { s.get_vertices(atlas) }).collect()
    }
}