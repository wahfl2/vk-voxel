use crate::{render::{texture::TextureAtlas, vertex::VertexRaw}, world::block_data::StaticBlockData};

use super::quad::BlockQuad;

pub trait ChunkRender {
    fn get_render_section(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> RenderSection;
}

impl<T> ChunkRender for [T]
where 
    T: ChunkRender
{
    fn get_render_section(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> RenderSection {
        let sections_iter = self.iter().map(|r| { 
            r.get_render_section(atlas, block_data) 
        });

        let (mut len1, mut len2) = (0, 0);
        sections_iter.clone().for_each(|s| { 
            len1 += s.block_quads.len();
            len2 += s.deco_vertices.len();
        });

        let (mut q, mut v) = (Vec::with_capacity(len1), Vec::with_capacity(len2));
        sections_iter.for_each(|mut s| {
            q.append(&mut s.block_quads);
            v.append(&mut s.deco_vertices);
        });

        RenderSection {
            block_quads: q,
            deco_vertices: v,
        }
    }
}

#[derive(Clone)]
pub struct RenderSection {
    pub block_quads: Vec<BlockQuad>,
    pub deco_vertices: Vec<VertexRaw>,
}

impl RenderSection {
    pub fn empty() -> Self {
        Self {
            block_quads: Vec::new(),
            deco_vertices: Vec::new(),
        }
    }
}