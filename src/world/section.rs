use std::array;

use ndarray::{Array3, arr3};
use ultraviolet::{UVec3, Vec3, IVec3};

use crate::{render::{util::Reversed, brick::brickmap::Brickmap}, util::util::Facing};

use super::{block_access::BlockAccess, block_data::{BlockHandle, StaticBlockData, Blocks}};


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
    pub brickmap: Brickmap,
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

            brickmap: Brickmap::empty(),
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

    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(|b| *b == Blocks::Air.handle())
    }

    pub fn flat_iter(&self) -> impl Iterator<Item = (UVec3, &BlockHandle)> {
        self.blocks.indexed_iter()
            .map(|((x, y, z), b)| { ((x as u32, y as u32, z as u32).into(), b) })
    }

    pub fn update_brickmap(&mut self, block_data: &StaticBlockData) {
        self.brickmap.solid_mask = self.solid_mask(block_data);
    }

    pub fn solid_mask(&self, block_data: &StaticBlockData) -> [[u8; 8]; 8] {
        if SECTION_SIZE != UVec3::new(8, 8, 8) {
            panic!("Section size incompatible");
        }

        let mut ret = [[0; 8]; 8];

        for ((x, y, z), b) in self.blocks.indexed_iter() {
            if block_data.get(b).model.is_full() {
                ret[x][y] |= 1 << z;
            }
        }

        return ret;
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