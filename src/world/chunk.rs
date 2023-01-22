use std::array;
use ultraviolet::{IVec2, UVec3, Vec2, Vec3};

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
        // Horrid
        let mut models = Vec::new();
        let xz = Vec2::from(self.pos * 16) + Vec2::new(0.5, 0.5);
        let base = Vec3::new(xz.x, 0.5, xz.y);

        for (i, section) in self.sections.iter().enumerate() {
            let y_add = i as f32;
            for (relative_pos, block) in section.flat_iter() {
                if let Some(mut model) = block_data.get(block).model {
                    let section_add = base + (Vec3::unit_y() * y_add) + Vec3::from(relative_pos);
                    model.center += section_add;
                    models.push(model);
                }
            }
        }

        models.get_vertices(atlas, block_data)
    }
}