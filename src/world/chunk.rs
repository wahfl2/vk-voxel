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

    pub fn cull_inner(&mut self, section_range: impl RangeBounds<usize>, block_data: &StaticBlockData) {
        let range = Self::get_section_range(section_range);
        for i in range {
            self.sections[i].cull_inner(block_data);
            if i > 0 {
                let plane = self.sections[i - 1].blocks.index_axis(Axis(1), SIZE_SUB1.y).to_owned();
                self.sections[i].cull_outer(Facing::DOWN, &plane, block_data);
            }

            if i < CH_USIZE - 1 {
                let plane = self.sections[i + 1].blocks.index_axis(Axis(1), 0).to_owned();
                self.sections[i].cull_outer(Facing::UP, &plane, block_data);
            }
        }
    }

    pub fn cull_adjacent(
        &mut self, 
        dir: Facing, 
        adjacent_chunk: &Chunk, 
        section_range: impl RangeBounds<usize>, 
        block_data: &StaticBlockData
    ) {
        // TODO: replace Facing with a 2D variant here
        if dir == Facing::UP || dir == Facing::DOWN { panic!("Wrong Facing dummy.") }

        let range = Self::get_section_range(section_range);
        for i in range {
            let inner_section = self.sections.get_mut(i).unwrap();
            let outer_section = &adjacent_chunk.sections[i];

            let outer_data = match (dir.axis, dir.sign) {
                (crate::util::util::Axis::X, Sign::Positive) => outer_section.blocks.index_axis(Axis(0), 0),
                (crate::util::util::Axis::X, Sign::Negative) => outer_section.blocks.index_axis(Axis(0), SIZE_SUB1.x),
                (crate::util::util::Axis::Z, Sign::Positive) => outer_section.blocks.index_axis(Axis(2), 0),
                (crate::util::util::Axis::Z, Sign::Negative) => outer_section.blocks.index_axis(Axis(2), SIZE_SUB1.z),
                _ => unreachable!()
            }.to_owned();

            inner_section.cull_outer(dir, &outer_data, block_data);
        }
    }

    fn get_section_range(r: impl RangeBounds<usize>) -> RangeInclusive<usize> {
        let start = match r.start_bound() {
            std::ops::Bound::Included(n) => *n,
            std::ops::Bound::Excluded(n) => n + 1,
            std::ops::Bound::Unbounded => 0,
        };

        let end = match r.end_bound() {
            std::ops::Bound::Included(n) => *n,
            std::ops::Bound::Excluded(n) => n - 1,
            std::ops::Bound::Unbounded => CH_USIZE - 1,
        };

        if start > CH_USIZE - 1 || end > CH_USIZE - 1 { panic!("Section range out of bounds.") }
        start..=end
    }

    pub fn init_mesh(&mut self, block_data: &StaticBlockData) {
        self.cull_inner(.., block_data);
    }

    pub fn rebuild_mesh(&mut self, block_data: &StaticBlockData) {
        self.sections.iter_mut().enumerate().for_each(|(i, section)| {
            let offset = IVec3::new(self.pos.x, i as i32, self.pos.y) * I_SECTION_SIZE;
            section.rebuild_mesh(
                offset.into(), 
                block_data
            );
        });
    }
}

impl ChunkRender for &Chunk {
    fn get_render_section(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> crate::render::mesh::chunk_render::RenderSection {
        self.sections.get_render_section(atlas, block_data)
    }
}