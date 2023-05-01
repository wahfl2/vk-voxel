use std::{ops::{RangeBounds, RangeInclusive}, array};

use ndarray::Axis;
use ultraviolet::{IVec2, IVec3};

use crate::{render::{mesh::chunk_render::ChunkRender, texture::TextureAtlas}, util::{util::{Facing, Sign}, more_vec::UsizeVec3}};

use super::{section::{Section, I_SECTION_SIZE, SECTION_SIZE}, block_data::StaticBlockData, generation::terrain::TerrainGenerator};

pub const CHUNK_HEIGHT: u32 = 32;
const CH_USIZE: usize = CHUNK_HEIGHT as usize;

pub struct Chunk {
    pub pos: IVec2,
    pub sections: Box<[Section; CH_USIZE]>,
}

const SIZE_SUB1: UsizeVec3 = UsizeVec3::new(
    (SECTION_SIZE.x - 1) as usize,
    (SECTION_SIZE.y - 1) as usize,
    (SECTION_SIZE.z - 1) as usize,
);

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