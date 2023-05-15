use std::array;

use ultraviolet::IVec2;

use super::{section::Section, block_data::StaticBlockData, generation::terrain::TerrainGenerator};

pub const CHUNK_HEIGHT: u32 = 32;
const CH_USIZE: usize = CHUNK_HEIGHT as usize;

#[derive(Debug)]
pub struct Chunk {
    pub pos: IVec2,
    pub sections: Box<[Section; CH_USIZE]>,
}

impl Chunk {
    pub fn empty(pos: IVec2) -> Self {
        let sections = Box::new(array::from_fn(|_| { Section::empty() }));
        Self { pos, sections }
    }

    pub fn generate(pos: IVec2, generator: &mut TerrainGenerator) -> Self {
        generator.gen_chunk(pos)
    }

    pub fn update_brickmap(&mut self, block_data: &StaticBlockData) {
        for section in self.sections.iter_mut() {
            section.update_brickmap(block_data);
        }
    }
}