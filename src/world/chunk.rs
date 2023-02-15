use std::ops::{RangeBounds, RangeInclusive};

use ndarray::{s, Axis};
use rayon::prelude::{ParallelIterator, IntoParallelRefMutIterator, IndexedParallelIterator, IntoParallelIterator};
use ultraviolet::{IVec2, UVec3, IVec3};

use crate::{render::{mesh::{renderable::Renderable, chunk_render::ChunkRender, quad::BlockQuad}, texture::TextureAtlas, vertex::VertexRaw}, util::util::{Facing, Sign}};

use super::{section::Section, block_access::BlockAccess, block_data::{BlockHandle, StaticBlockData}, generation::terrain::TerrainGenerator};

pub struct Chunk {
    pub pos: IVec2,
    pub sections: Vec<Section>,
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

    pub fn generate(pos: IVec2, generator: &TerrainGenerator) -> Self {
        let mut collect = Vec::with_capacity(16);
        (0..16).into_par_iter().map(|section_y| {
            let offset = IVec3::new(pos.x * 16, section_y * 16, pos.y * 16);
            generator.gen_section(offset)
        }).collect_into_vec(&mut collect);
        Self { pos, sections: collect }
    }

    pub fn cull_inner(&mut self, section_range: impl RangeBounds<usize>, block_data: &StaticBlockData) {
        let range = Self::get_section_range(section_range);
        for i in range {
            self.sections[i].cull_inner(block_data);
            if i > 0 {
                let plane = self.sections[i - 1].blocks.index_axis(Axis(1), 15).to_owned();
                self.sections[i].cull_outer(Facing::DOWN, &plane, block_data);
            }

            if i < 15 {
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
                (crate::util::util::Axis::X, Sign::Negative) => outer_section.blocks.index_axis(Axis(0), 15),
                (crate::util::util::Axis::Z, Sign::Positive) => outer_section.blocks.index_axis(Axis(2), 0),
                (crate::util::util::Axis::Z, Sign::Negative) => outer_section.blocks.index_axis(Axis(2), 15),
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
            std::ops::Bound::Unbounded => 15,
        };

        if start > 15 || end > 15 { panic!("Section range out of bounds.") }
        start..=end
    }

    pub fn init_mesh(&mut self, block_data: &StaticBlockData) {
        self.cull_inner(.., block_data);
    }

    pub fn rebuild_mesh(&mut self, atlas: &TextureAtlas, block_data: &StaticBlockData) {
        self.sections.par_iter_mut().enumerate().for_each(|(i, section)| {
            let offset = IVec3::new(self.pos.x * 16, i as i32 * 16, self.pos.y * 16);
            section.rebuild_mesh(
                offset.into(), 
                atlas, 
                block_data
            );
        });
    }

    fn fill_column(&mut self, x: usize, z: usize, height: u32, fill_block: BlockHandle) {
        let column_iter = self.sections.iter_mut().flat_map(
            |s| { s.blocks.slice_mut(s![x, .., z]) }
        );

        for (i, block) in column_iter.enumerate() {
            if i > height as usize { break }
            *block = fill_block;
        }
    }
}

impl ChunkRender for &Chunk {
    fn get_render_section(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> crate::render::mesh::chunk_render::RenderSection {
        self.sections.get_render_section(atlas, block_data)
    }
}