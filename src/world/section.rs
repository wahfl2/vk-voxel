use std::{array, ops::Index};

use ndarray::{Array3, arr3, Axis, Array2};
use ultraviolet::{UVec3, Vec3, IVec2, IVec3};

use crate::{render::{mesh::chunk_render::{ChunkRender, RenderSection}, texture::TextureAtlas, util::Reversed}, util::{util::{Facing, Sign, UVecToSigned, VecAxisIndex}, more_vec::UsizeVec3}};

use super::{block_access::BlockAccess, block_data::{BlockHandle, StaticBlockData, BlockType, ModelType}};


pub const SECTION_SIZE: UVec3 = UVec3::new(8, 8, 8);

pub const I_SECTION_SIZE: IVec3 = IVec3::new(
    SECTION_SIZE.x as i32,
    SECTION_SIZE.y as i32,
    SECTION_SIZE.z as i32,
);

pub const F_SECTION_SIZE: Vec3 = Vec3::new(
    SECTION_SIZE.x as f32,
    SECTION_SIZE.y as f32,
    SECTION_SIZE.z as f32,
);

pub struct Section {
    pub blocks: Array3<BlockHandle>,
    pub cull: Array3<BlockCull>,
    pub render: RenderSection,
}

impl BlockAccess for Section {
    fn get_block(&self, pos: UVec3) -> BlockHandle {
        *self.blocks.get((pos.x as usize, pos.y as usize, pos.z as usize)).unwrap()
    }

    fn set_block(&mut self, pos: UVec3, block: BlockHandle) {
        *self.blocks.get_mut((pos.x as usize, pos.y as usize, pos.z as usize)).unwrap() = block;
    }
}

impl Section {
    pub fn empty() -> Self {
        Self {
            blocks: arr3(&[[[BlockHandle::default(); 
                SECTION_SIZE.x as usize]; 
                SECTION_SIZE.y as usize]; 
                SECTION_SIZE.z as usize]
            ),

            cull: arr3(&[[[BlockCull::none(); 
                SECTION_SIZE.x as usize]; 
                SECTION_SIZE.y as usize]; 
                SECTION_SIZE.z as usize]
            ),

            render: RenderSection::empty(),
        }
    }

    pub fn full(block: BlockHandle) -> Self {
        Self {
            blocks: arr3(&[[[block; 
                SECTION_SIZE.x as usize]; 
                SECTION_SIZE.y as usize]; 
                SECTION_SIZE.z as usize]
            ),

            ..Self::empty()
        }
    }

    pub fn flat_iter(&self) -> impl Iterator<Item = (UVec3, &BlockHandle)> {
        self.blocks.indexed_iter()
            .map(|((x, y, z), b)| { ((x as u32, y as u32, z as u32).into(), b) })
    }

    pub fn cull_inner(&mut self, block_data: &StaticBlockData) {
        let iter = self.blocks.indexed_iter().map(|(p, b)| { (UsizeVec3::from(p), b) });

        for (pos, block) in iter {
            if block_data.get(block).block_type == BlockType::None { continue; }
            for (face, n) in self.get_neighbors(pos).into_iter().enumerate() {
                match n {
                    Neighbor::Block(b) => {
                        let cull = self.cull.get_mut((pos.x, pos.y, pos.z)).unwrap();
                        if block_data.get(&b).block_type == BlockType::Full {
                            cull.set_face(face, true);
                        } else {
                            cull.set_face(face, false);
                        }
                    },
                    _ => (),
                }
            }
        }
    }

    pub fn cull_outer(&mut self, dir: Facing, outer_data: &Array2<BlockHandle>, block_data: &StaticBlockData) {
        let face_num = dir.to_num();
        let axis = match dir.axis {
            crate::util::util::Axis::X => Axis(0),
            crate::util::util::Axis::Y => Axis(1),
            crate::util::util::Axis::Z => Axis(2),
        };

        let depth = match dir.sign {
            Sign::Positive => SECTION_SIZE.get(axis) as usize - 1,
            Sign::Negative => 0,
        };

        let plane = self.blocks.index_axis(axis, depth);
        let mut cull_plane = self.cull.index_axis_mut(axis, depth);
        let iter = plane.indexed_iter().zip(cull_plane.iter_mut().zip(outer_data.iter()));

        for ((_pos, inner_block), (cull, outer_block)) in iter {
            if block_data.get(inner_block).block_type == BlockType::None { continue; }
            cull.set_face(face_num, block_data.get(outer_block).block_type == BlockType::Full);
        }
    }

    pub fn cull_all_outer(&mut self, outer_data: [Option<Array2<BlockHandle>>; 6], block_data: &StaticBlockData) {
        for i in 0..6 {
            if let Some(outer) = &outer_data[i] {
                self.cull_outer(Facing::from_num(i), outer, block_data);
            }
        }
    }

    pub fn cull_all(&mut self, outer_data: [Option<Array2<BlockHandle>>; 6], block_data: &StaticBlockData) {
        self.cull_inner(block_data);
        self.cull_all_outer(outer_data, block_data);
    }

    pub fn rebuild_mesh(
        &mut self, 
        offset: Vec3, 
        block_data: &StaticBlockData,
    ) {
        let blocks = self.blocks.indexed_iter().map(|(p, b)| { (UsizeVec3::from(p), b) }).collect::<Vec<_>>();

        self.render.block_quads.clear();
        self.render.deco_vertices.clear();

        let mut block_refs = Vec::new();
        let mut deco_refs = Vec::new();

        blocks.iter().for_each(|(pos, block)| {
            let data = block_data.get(block);
            if data.block_type == BlockType::None { return; }

            match &data.model {
                ModelType::FullBlock(m) => {
                    block_refs.push((m, pos));
                },

                ModelType::Plant(m) => {
                    deco_refs.push((m, pos));
                },
                _ => (),
            }
        });

        self.render.block_quads.extend(block_refs.into_iter().flat_map(|(m, pos)| {
            let faces = m.get_faces(offset + pos.into_vec3());
            let cull = self.cull.get((pos.x, pos.y, pos.z)).unwrap();

            cull.get_unculled().into_iter().map(move |i| { faces[i].into_block_quad() })
        }));

        self.render.deco_vertices.extend(deco_refs.into_iter().flat_map(|(m, pos)| {
            m.with_translation(offset + pos.into_vec3()).get_raw_vertices().collect::<Vec<_>>()
        }))
    }

    fn get_neighbors(&self, pos: UsizeVec3) -> [Neighbor; 6] {
        let mut n = [Neighbor::Boundary; 6];
        let m = UsizeVec3::from(SECTION_SIZE - UVec3::one());

        if pos.x < m.x { n[0] = Neighbor::Block(*self.blocks.get((pos.x + 1, pos.y, pos.z)).unwrap()) }
        if pos.x > 0   { n[1] = Neighbor::Block(*self.blocks.get((pos.x - 1, pos.y, pos.z)).unwrap()) }
        if pos.y < m.y { n[2] = Neighbor::Block(*self.blocks.get((pos.x, pos.y + 1, pos.z)).unwrap()) }
        if pos.y > 0   { n[3] = Neighbor::Block(*self.blocks.get((pos.x, pos.y - 1, pos.z)).unwrap()) }
        if pos.z < m.z { n[4] = Neighbor::Block(*self.blocks.get((pos.x, pos.y, pos.z + 1)).unwrap()) }
        if pos.z > 0   { n[5] = Neighbor::Block(*self.blocks.get((pos.x, pos.y, pos.z - 1)).unwrap()) }

        n
    }
}

#[derive(Copy, Clone, Debug)]
enum Neighbor {
    Boundary,
    Block(BlockHandle),
}

impl From<Option<BlockHandle>> for Neighbor {
    fn from(value: Option<BlockHandle>) -> Self {
        match value {
            Some(b) => Self::Block(b),
            None => Self::Boundary,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BlockCull {
    inner: u8,
}

impl BlockCull {
    pub fn none() -> Self {
        Self { inner: 0 }
    }

    /// The last six bits represent `+X -X +Y -Y +Z -Z`
    pub fn from_bits(bits: u8) -> Self {
        Self { inner: bits }
    }

    pub fn from_array(arr: [bool; 6]) -> Self {
        let mut bits = 0u8;
        for (i, b) in arr.reversed().iter().enumerate() {
            bits |= Self::to_u8(b) << i;
        }

        Self::from_bits(bits)
    }

    pub fn is_culled(&self, dir: Facing) -> bool {
        self.is_culled_num(dir.to_num())
    }

    fn is_culled_num(&self, num: usize) -> bool {
        self.inner & (1 << (6 - num)) > 0
    }

    pub fn set_face(&mut self, face: usize, b: bool) {
        let mask = 1 << (6 - face);
        self.inner = (self.inner & !mask) | (Self::to_u8(&b) << (6 - face));
    }

    pub fn get_bools(&self) -> [bool; 6] {
        array::from_fn(|face| {
            self.is_culled_num(face)
        })
    }

    pub fn get_unculled(&self) -> Vec<usize> {
        (0..6).filter_map(|f| { 
            match self.is_culled_num(f) {
                true => None,
                false => Some(f),
            }
        }).collect()
    }
    
    fn to_u8(b: &bool) -> u8 {
        match b {
            true => 1,
            false => 0,
        }
    }
}

impl ChunkRender for Section {
    fn get_render_section(&self, _atlas: &TextureAtlas, _block_data: &StaticBlockData) -> RenderSection {
        self.render.clone()
    }
}