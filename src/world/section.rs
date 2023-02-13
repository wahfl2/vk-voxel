use std::array;

use ndarray::{Array3, arr3, Axis, Array2};
use ultraviolet::{UVec3, Vec3, IVec3};

use crate::{render::{mesh::{chunk_render::{ChunkRender, RenderSection}, quad::BlockQuad}, texture::TextureAtlas, util::Reversed}, util::{util::{Facing, Sign}, more_vec::UsizeVec3}};

use super::{block_access::BlockAccess, block_data::{BlockHandle, StaticBlockData, BlockType, ModelType}, terrain::TerrainGenerator};

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
            blocks: arr3(&[[[BlockHandle::default(); 16]; 16]; 16]),
            cull: arr3(&[[[BlockCull::none(); 16]; 16]; 16]),
            render: RenderSection::empty(),
        }
    }

    pub fn generate(offset: IVec3, terrain_gen: &TerrainGenerator) -> Self {
        let arr: [[[BlockHandle; 16]; 16]; 16] = 
            array::from_fn(|x_off| {
                array::from_fn(|y_off| {
                    array::from_fn(|z_off| {
                        let pos = offset + (x_off as i32, y_off as i32, z_off as i32).into();
                        terrain_gen.gen_at(pos.into())
                    })
                })
            });

        Self {
            blocks: arr3(&arr),
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
            Sign::Positive => 15,
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
        atlas: &TextureAtlas, 
        block_data: &StaticBlockData,
    ) {
        let blocks = self.blocks.indexed_iter().map(|(p, b)| { (UsizeVec3::from(p), b) }).collect::<Vec<_>>();

        self.render.block_quads.clear();
        self.render.deco_vertices.clear();

        blocks.into_iter().for_each(|(pos, block)| {
            let data = block_data.get(block);
            if data.block_type == BlockType::None { return; }

            match data.model.clone() {
                ModelType::FullBlock(mut m) => {
                    m.center = offset + Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32);
                    let faces = m.get_faces();
                    let cull = self.cull.get((pos.x, pos.y, pos.z)).unwrap();

                    self.render.block_quads.extend(
                        cull.get_unculled().map(move |(i, _)| { faces[i].into_block_quad(atlas) })
                    );
                },

                ModelType::Plant(m) => {
                    self.render.deco_vertices.append(&mut m.with_translation(pos.into_vec3()).get_raw_vertices());
                },
                _ => (),
            }
        });
    }

    fn get_neighbors(&self, pos: UsizeVec3) -> [Neighbor; 6] {
        let mut n = [Neighbor::Boundary; 6];

        if pos.x < 15 { n[0] = Neighbor::Block(*self.blocks.get((pos.x + 1, pos.y, pos.z)).unwrap()) }
        if pos.x > 0  { n[1] = Neighbor::Block(*self.blocks.get((pos.x - 1, pos.y, pos.z)).unwrap()) }
        if pos.y < 15 { n[2] = Neighbor::Block(*self.blocks.get((pos.x, pos.y + 1, pos.z)).unwrap()) }
        if pos.y > 0  { n[3] = Neighbor::Block(*self.blocks.get((pos.x, pos.y - 1, pos.z)).unwrap()) }
        if pos.z < 15 { n[4] = Neighbor::Block(*self.blocks.get((pos.x, pos.y, pos.z + 1)).unwrap()) }
        if pos.z > 0  { n[5] = Neighbor::Block(*self.blocks.get((pos.x, pos.y, pos.z - 1)).unwrap()) }

        n
    }

    fn neighbor_block(&self, pos: UsizeVec3) -> Neighbor {
        Neighbor::Block(*self.blocks.get((pos.x, pos.y, pos.z)).unwrap())
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

    pub fn get_unculled(&self) -> impl Iterator<Item = (usize, bool)> + '_ {
        (0..6).map(|f| { (f, self.is_culled_num(f)) }).filter(|(_, c)| { !c })
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