use ultraviolet::{IVec2, UVec3, IVec3};

use crate::render::{mesh::renderable::Renderable, texture::TextureAtlas, vertex::VertexRaw};

use super::{section::Section, block_access::BlockAccess, block_data::{BlockHandle, StaticBlockData}, terrain::TerrainGenerator};

pub struct Chunk {
    pub pos: IVec2,
    sections: Vec<Section>,
}

impl BlockAccess for Chunk {
    fn get_block(&self, pos: UVec3) -> BlockHandle {
        let relative_y = pos.y % 16;
        let section_idx = (pos.y / 16) as usize;
        self.sections[section_idx].get_block(UVec3::new(pos.x, relative_y, pos.z))
    }

    fn set_block(&mut self, pos: UVec3, block: BlockHandle) {
        let relative_y = pos.y % 16;
        let section_idx = (pos.y / 16) as usize;
        self.sections[section_idx].set_block(UVec3::new(pos.x, relative_y, pos.z), block);
    }
}

impl Chunk {
    pub fn empty(pos: IVec2) -> Self {
        let sections = Vec::from_iter((0..16).into_iter().map(|_| { Section::empty() }));
        Self { pos, sections }
    }

    pub fn gen(&mut self, generator: &TerrainGenerator) {
        let x_range = (self.pos.x * 16)..((self.pos.x+1) * 16);
        let z_range = (self.pos.y * 16)..((self.pos.y+1) * 16);
        for (rel_x, x) in x_range.enumerate() {
            for (rel_z, z) in z_range.clone().enumerate() {
                self.fill_column(rel_x, rel_z, generator.get_height((x, z).into()), generator.fill_block)
            }
        }
    }

    pub fn rebuild_mesh(&mut self, atlas: &TextureAtlas, block_data: &StaticBlockData) {
        self.sections.iter_mut().enumerate().for_each(|(i, section)| {
            let offset = IVec3::new(self.pos.x * 16, i as i32 * 16, self.pos.y * 16);
            section.rebuild_mesh(offset.into(), atlas, block_data);
        });
    }

    fn fill_column(&mut self, x: usize, z: usize, height: u32, fill_block: BlockHandle) {
        let column_iter = self.sections.iter_mut().flat_map(
            |s| { s.column_iter_mut(x, z) }
        );

        for (i, block) in column_iter.enumerate() {
            if i > height as usize { break }
            *block = fill_block;
        }
    }
}

impl Renderable for &Chunk {
    fn get_vertices(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> Vec<VertexRaw> {
        self.sections.get_vertices(atlas, block_data)
    }
}